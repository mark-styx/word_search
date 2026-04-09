type MessageHandler = (data: unknown) => void;

interface WSOptions {
  url: string;
  token: string;
  gameId: string;
  onMessage: MessageHandler;
  onOpen?: () => void;
  onClose?: () => void;
  onError?: (err: Event) => void;
}

const MAX_RECONNECT_ATTEMPTS = 5;
const BASE_RECONNECT_DELAY_MS = 1000;

export class GameSocket {
  private ws: WebSocket | null = null;
  private attempts = 0;
  private disposed = false;
  private opts: WSOptions;

  constructor(opts: WSOptions) {
    this.opts = opts;
    this.connect();
  }

  private get url(): string {
    const base = this.opts.url.replace(/^http/, "ws");
    return `${base}/ws?token=${this.opts.token}&game_id=${this.opts.gameId}`;
  }

  private connect() {
    if (this.disposed) return;
    this.ws = new WebSocket(this.url);

    this.ws.onopen = () => {
      this.attempts = 0;
      this.opts.onOpen?.();
    };

    this.ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        this.opts.onMessage(data);
      } catch {
        // ignore malformed messages
      }
    };

    this.ws.onclose = () => {
      this.opts.onClose?.();
      this.tryReconnect();
    };

    this.ws.onerror = (err) => {
      this.opts.onError?.(err);
    };
  }

  private tryReconnect() {
    if (this.disposed || this.attempts >= MAX_RECONNECT_ATTEMPTS) return;
    this.attempts++;
    const delay = BASE_RECONNECT_DELAY_MS * Math.pow(2, this.attempts - 1);
    setTimeout(() => this.connect(), delay);
  }

  send(data: unknown) {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(data));
    }
  }

  close() {
    this.disposed = true;
    this.ws?.close();
    this.ws = null;
  }
}
