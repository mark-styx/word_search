import { QRCodeSVG } from "qrcode.react";

import { Modal, Select } from "@/components/ui";

type ShareOption = {
  value: string;
  label: string;
};

type ShareLobbyModalProps = {
  open: boolean;
  shareOptions: ShareOption[];
  shareTarget: string;
  shareUrl: string;
  sharingSpecificGame: boolean;
  onClose: () => void;
  onChangeTarget: (target: string) => void;
};

export default function ShareLobbyModal({
  open,
  shareOptions,
  shareTarget,
  shareUrl,
  sharingSpecificGame,
  onClose,
  onChangeTarget,
}: ShareLobbyModalProps) {
  return (
    <Modal
      open={open}
      onClose={onClose}
      title={sharingSpecificGame ? "Share Game" : "Share Server"}
    >
      <div className="flex flex-col items-center gap-4">
        {shareOptions.length > 1 && (
          <div className="w-full">
            <Select
              label="Share Target"
              options={shareOptions}
              value={shareTarget}
              onChange={(event) => onChangeTarget(event.target.value)}
            />
          </div>
        )}
        {shareUrl && <QRCodeSVG value={shareUrl} size={180} />}
        <p className="text-center text-sm text-text-secondary">
          {sharingSpecificGame
            ? "Scan to open this game directly on another device"
            : "Scan to open this server on another device"}
        </p>
        <p className="text-xs font-mono text-text-secondary">{shareUrl}</p>
      </div>
    </Modal>
  );
}
