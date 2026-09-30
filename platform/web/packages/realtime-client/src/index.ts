export type MessageHandler<T = any> = (data: T) => void;

export interface RealtimeClientOptions {
  url: string;
  reconnectIntervalMs?: number;
  maxReconnectIntervalMs?: number;
  maxQueueSize?: number;
}

export class RealtimeClient {
  private url: string;
  private ws: WebSocket | null = null;
  private reconnectInterval: number;
  private maxReconnectInterval: number;
  private currentDelay: number;
  private maxQueueSize: number;
  private sendQueue: string[] = [];
  private handlers: Map<string, Set<MessageHandler>> = new Map();
  private isClosedExplicitly: boolean = false;

  constructor(opts: RealtimeClientOptions) {
    this.url = opts.url;
    this.reconnectInterval = opts.reconnectIntervalMs || 1000;
    this.maxReconnectInterval = opts.maxReconnectIntervalMs || 30000;
    this.currentDelay = this.reconnectInterval;
    this.maxQueueSize = opts.maxQueueSize || 1000;
  }

  public connect(): void {
    if (typeof window === "undefined" || !("WebSocket" in window)) return;
    this.isClosedExplicitly = false;

    try {
      this.ws = new WebSocket(this.url);
      this.ws.onopen = () => {
        this.currentDelay = this.reconnectInterval;
        this.emit("open", null);
        this.flushQueue();
      };

      this.ws.onmessage = (event) => {
        try {
          const parsed = JSON.parse(event.data);
          const eventType = parsed.type || "message";
          this.emit(eventType, parsed.data || parsed);
        } catch {
          this.emit("raw", event.data);
        }
      };

      this.ws.onclose = () => {
        this.emit("close", null);
        if (!this.isClosedExplicitly) {
          this.scheduleReconnect();
        }
      };

      this.ws.onerror = (err) => {
        this.emit("error", err);
      };
    } catch (e) {
      this.scheduleReconnect();
    }
  }

  public on<T = any>(event: string, handler: MessageHandler<T>): () => void {
    if (!this.handlers.has(event)) {
      this.handlers.set(event, new Set());
    }
    this.handlers.get(event)!.add(handler);
    return () => this.handlers.get(event)?.delete(handler);
  }

  private emit(event: string, data: any): void {
    const list = this.handlers.get(event);
    if (list) {
      list.forEach((h) => h(data));
    }
  }

  public send(type: string, payload: any): void {
    const msg = JSON.stringify({ type, data: payload, ts: Date.now() });
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(msg);
    } else {
      if (this.sendQueue.length < this.maxQueueSize) {
        this.sendQueue.push(msg);
      }
    }
  }

  private flushQueue(): void {
    while (this.sendQueue.length > 0 && this.ws && this.ws.readyState === WebSocket.OPEN) {
      const msg = this.sendQueue.shift();
      if (msg) this.ws.send(msg);
    }
  }

  private scheduleReconnect(): void {
    setTimeout(() => {
      this.currentDelay = Math.min(this.currentDelay * 1.5, this.maxReconnectInterval);
      this.connect();
    }, this.currentDelay);
  }

  public disconnect(): void {
    this.isClosedExplicitly = true;
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
  }
}
