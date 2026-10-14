import { getAccessToken } from "./api";

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

const BASE_RECONNECT_DELAY_MS = 1000;
const MAX_RECONNECT_DELAY_MS = 10_000;
const MAX_QUEUED_MESSAGES = 32;

export class GameSocket {
  private ws: WebSocket | null = null;
  private attempts = 0;
  private disposed = false;
  private opts: WSOptions;
  private queuedMessages: string[] = [];
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(opts: WSOptions) {
    this.opts = opts;
    this.connect();
  }

  private get url(): string | null {
    const token = getAccessToken() ?? this.opts.token;
    if (!token) {
      return null;
    }
    const base = this.opts.url.replace(/^http/, "ws");
    return `${base}/ws?token=${encodeURIComponent(token)}&game_id=${this.opts.gameId}`;
  }

  private connect() {
    if (this.disposed) return;
    const url = this.url;
    if (!url) {
      this.disposeSocket();
      this.opts.onClose?.();
      return;
    }

    this.ws = new WebSocket(url);

    this.ws.onopen = () => {
      this.attempts = 0;
      this.clearReconnectTimer();
      this.flushQueue();
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
    if (this.disposed) return;
    this.attempts++;
    const delay = Math.min(
      BASE_RECONNECT_DELAY_MS * Math.pow(2, this.attempts - 1),
      MAX_RECONNECT_DELAY_MS
    );
    this.clearReconnectTimer();
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, delay);
  }

  private flushQueue() {
    if (this.ws?.readyState !== WebSocket.OPEN || this.queuedMessages.length === 0) {
      return;
    }

    for (const message of this.queuedMessages) {
      this.ws.send(message);
    }
    this.queuedMessages = [];
  }

  private clearReconnectTimer() {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private disposeSocket() {
    this.ws = null;
  }

  send(data: unknown) {
    const serialized = JSON.stringify(data);

    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(serialized);
      return;
    }

    this.queuedMessages.push(serialized);
    if (this.queuedMessages.length > MAX_QUEUED_MESSAGES) {
      this.queuedMessages.shift();
    }

    if (!this.ws || this.ws.readyState === WebSocket.CLOSED) {
      this.tryReconnect();
    }
  }

  close() {
    this.disposed = true;
    this.clearReconnectTimer();
    this.queuedMessages = [];
    this.ws?.close();
    this.disposeSocket();
  }

  reconnectNow() {
    if (this.disposed) {
      return;
    }

    this.clearReconnectTimer();
    this.ws?.close();
    this.disposeSocket();
    this.connect();
  }
}
