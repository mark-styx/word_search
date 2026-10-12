# EC2/ECS → Vultr Migration Plan

*Drafted 2026-10-12. Data sources: live AWS inventory + Cost Explorer (Sept 2026), Vultr `/v2/plans` API (fetched today).*

## What actually runs on AWS today

Verified against the account — not from memory:

| Resource | Detail | Cost/mo |
|---|---|---|
| EC2 `i-02c975be8bdb26c45` (`word-search-prod-host`) | t3.small, us-east-2, running since Apr 2026 | $14.96 |
| EBS gp3 ×2 (20GB root + 30GB data) | both attached | $4.00 |
| Public IPv4 (EIP 3.147.1.133) | the new-ish AWS IPv4 tax | $3.60 |
| Route 53 — 8 hosted zones | cheesy-games.com now on Cloudflare, zone deletable | $4.00 |
| DNS queries | | $0.05 |
| ECS / RDS / ELB | **nothing running.** Sentient-news CDK stack (ECS Fargate ×2 services, RDS Postgres t4g, ALB, NAT) was briefly live in July, torn down. ~$1.50 lifetime. | $0 |
| **Total** | | **$26.61** |

July also included $31 Amazon Registrar (cheesy-games.com renewal — annual, independent of this migration).

What's on the box: Caddy + word_search blue/green Docker stacks (Rust backend, Next.js frontend, SQLite volumes) + preview stack + the blue-green scripts. Scrabble stays home-LAN. sector-defense is already fully on Cloudflare Workers/D1 and does not move.

## Vultr target

Prices verified from Vultr's public plans API today. 1 free IPv4 per instance, NVMe on the faster lines, transfer pooled account-wide with free ingress.

| Plan | Spec | Disk | Transfer | $/mo |
|---|---|---|---|---|
| `vhp-1c-2gb` | 1 vCPU / 2GB | 50GB | 3TB | $12 |
| **`vhp-2c-2gb` (recommended start)** | 2 vCPU / 2GB | 60GB | 4TB | $18 |
| `vhp-2c-4gb` (sentient-news later) | 2 vCPU / 4GB | 100GB | 5TB | $24 |

`t3.small` = 2 burst vCPU / 2GB, so `vhp-2c-2gb` is exact spec parity (and non-bursting). `vhp-1c-2gb` is the likely-fine downsize once you see real load — the only spikes are blue-green image builds.

## Cost savings

**Today (word_search box + DNS):**

| | AWS now | Vultr 1c/2gb | Vultr 2c/2gb |
|---|---|---|---|
| Box (compute+disk+IPv4) | $22.56 | $12.00 | $18.00 |
| DNS | $4.05 R53 | $0 (move 7 remaining zones to Cloudflare, free) or keep R53 $3.55 | same |
| **Monthly** | **$26.61** | **$12–15.55** | **$18–21.55** |
| Savings | — | **$11–15/mo (42–55%)** | **$5–9/mo (17–32%)** |

Deleting the now-dead cheesy-games.com R53 zone alone is −$0.50 regardless.

**Future (when sentient-news ships):** the honest headline. Deployed as written in its CDK stack — 4 Fargate tasks, RDS db.t4g.small, ALB, NAT gateway — it bills ~$150–200/mo. Its `docker-compose.production.yml` (postgres + litellm + backend + frontend) runs whole on one `vhp-2c-4gb` at **$24/mo**. Avoided cost: **~$130–175/mo (~$1,600–2,100/yr)**. The ECS plan was designed for traffic sentient-news doesn't have.

**Annual summary:**
- word_search move: saves ~$100–175/yr
- never deploying the ECS/RDS architecture: avoids ~$1,600+/yr
- optional: transfer cheesy-games.com registration to Cloudflare at-cost (~$10.44/yr) vs Amazon's $31 — saves ~$20/yr, possible 60 days after the July renewal
- end state: AWS account at $0/mo (or ~$3.55/mo if the 7 idle zones stay parked in R53)

Straight talk: if word_search savings were the only motive, staying on AWS costs just $5–9/mo more with zero migration work. The real case for the move: AWS bill goes to zero (no surprise-billing surface left), flat predictable billing, and one Linux box as the estate's only stateful host — which was the architecture decision already made. Vultr completes "Cloudflare front door + data plane, one box for stateful."

## Migration plan (word_search EC2 → Vultr)

Ops note: this Mac has no SSH to the EC2 box; the box is managed from elsewhere (Mac Studio). Data-copy steps run from there, or via EC2 Session Manager. Add this Mac's pubkey when provisioning the Vultr box so both manage it.

### Phase 0 — provision (no downtime, ~30 min)
1. Vultr account, add SSH keys. Check for new-account credit promos.
2. Create `vhp-2c-2gb`, Ubuntu 24.04, region Chicago (closest analog to us-east-2; origin location matters little behind the CF edge). ~2 min to boot.
3. Harden: non-root user, `ufw` (22, 80, 443), unattended-upgrades, fail2ban.
4. Install Docker + compose plugin (the repo's documented bootstrap).
5. `git clone` word_search.

### Phase 1 — build shadow (no downtime, ~1 hr)
6. From EC2 (via Mac Studio): copy `.bluegreen/` state, Caddy config, and `/var/lib/caddy` (LE certs + state — avoids HTTP-01 issuance problems behind orange-cloud).
7. Restore latest nightly backup into the stack volumes (`backup-db.sh` output includes the db+wal+shm triple).
8. Build both stacks, start active one.
9. Smoke test without touching DNS: `curl --resolve wordsearch.cheesy-games.com:443:<new-ip> https://wordsearch.cheesy-games.com` — puzzle list, login, one save.

### Phase 2 — cutover (~2–5 min visible)
10. Stop Caddy on EC2 (freeze writes; site 502s briefly through CF).
11. Final DB sync EC2 → Vultr (same docker-cp triple procedure as promote).
12. Cloudflare dashboard: A record `wordsearch` 3.147.1.133 → new IP, keep proxied. Effective near-instantly at the edge.
13. Verify live: HTTPS 200, play a round, save a score, check it lands in the new box's DB.
14. Rollback path for one week = flip the A record back (EC2 stays running).

### Phase 3 — decommission + consolidate (after ~1 week green)
15. Terminate `i-02c975be8bdb26c45`, delete both EBS volumes, release the EIP.
16. Delete R53 zone `cheesy-games.com` (Cloudflare is authoritative now).
17. Optional: move the 7 remaining zones (markstyx.com, sentient-news.com, etc.) to Cloudflare — same NS-swap procedure as before, ~5 min each, −$3.55/mo.
18. Install the nightly backup cron on the Vultr box (line in `backup-db.sh` header). **Also** push nightly backups off-box (rclone to B2/R2, pennies) — on-box-only backups repeat the same-fate risk that already existed on EC2.
19. AWS → $0/mo. (Registrar $31/yr remains until/unless registration transfers.)

### Risks
- **LE certs:** copy `/var/lib/caddy` rather than re-issuing behind orange-cloud; fallback is grey-cloud during first issuance.
- **1 vCPU downsize:** fine for traffic, tight during parallel blue+green builds — stay on 2c/2gb until observed.
- **Same-fate backups:** addressed in step 18.
- **Transfer:** 3–4TB included vs word_search's ~GB-scale egress — a non-issue.
- **Latency:** wordsearch already served through the CF edge; users won't notice the origin moving from Ohio to Chicago.

### Sentient-news, when it ships (not now)
One `vhp-2c-4gb` ($24/mo) running the existing `docker-compose.production.yml` on the same Vultr account. Delete or archive the CDK stack — it encodes an architecture that costs 6–8× more for load that doesn't exist. If it ever outgrows one box, that's when a managed-DB / multi-host conversation is worth having.

## Bottom line

| Scenario | Now | After | Δ |
|---|---|---|---|
| word_search host + DNS | $26.61/mo AWS | $12–18/mo Vultr (+$0 DNS on CF) | −$5–15/mo |
| AWS account total | $26.61/mo + $31/yr registrar | $0/mo (+~$10/yr if registrar moves) | surprise-billing surface → zero |
| sentient-news (future) | ~$150–200/mo as CDK'd | $24/mo | avoid ~$1,600+/yr |
