export interface FramePlayerOptions {
  canvas: HTMLCanvasElement;
  frameCount: number;
  baseUrl: string; // e.g. "/assets/frames"
  mobileBaseUrl?: string; // e.g. "/assets/frames_mobile"
  posterUrl: string;
  maxMemoryMb?: number;
}

export class FramePlayer {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D | null;
  private frameCount: number;
  private baseUrl: string;
  private posterUrl: string;
  private images: Map<number, HTMLImageElement> = new Map();
  private currentFrame: number = 1;
  private isReducedMotion: boolean = false;

  constructor(opts: FramePlayerOptions) {
    this.canvas = opts.canvas;
    this.ctx = opts.canvas.getContext("2d");
    this.frameCount = opts.frameCount;
    this.baseUrl = opts.baseUrl;
    this.posterUrl = opts.posterUrl;

    if (typeof window !== "undefined") {
      this.isReducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    }
  }

  public async init(): Promise<void> {
    if (this.isReducedMotion) {
      await this.loadPoster();
      return;
    }

    // Preload first 20 frames immediately
    const initialBatch = Math.min(20, this.frameCount);
    const promises: Promise<void>[] = [];
    for (let i = 1; i <= initialBatch; i++) {
      promises.push(this.loadFrame(i));
    }
    await Promise.all(promises);
    this.render(1);

    // Lazily load remaining frames in background idle time
    if (typeof window !== "undefined" && "requestIdleCallback" in window) {
      (window as any).requestIdleCallback(() => this.preloadRemaining(initialBatch + 1));
    } else {
      setTimeout(() => this.preloadRemaining(initialBatch + 1), 100);
    }
  }

  private async loadFrame(idx: number): Promise<void> {
    if (this.images.has(idx)) return;
    return new Promise((resolve) => {
      const img = new Image();
      const numStr = String(idx).padStart(4, "0");
      img.src = `${this.baseUrl}/f_${numStr}.webp`;
      img.onload = () => {
        this.images.set(idx, img);
        resolve();
      };
      img.onerror = () => resolve();
    });
  }

  private async loadPoster(): Promise<void> {
    const img = new Image();
    img.src = this.posterUrl;
    img.onload = () => {
      if (this.ctx) {
        this.ctx.drawImage(img, 0, 0, this.canvas.width, this.canvas.height);
      }
    };
  }

  private async preloadRemaining(startIdx: number): Promise<void> {
    for (let i = startIdx; i <= this.frameCount; i++) {
      await this.loadFrame(i);
    }
  }

  public setProgress(progress: number): void {
    if (this.isReducedMotion) return;
    const clamped = Math.max(0, Math.min(1, progress));
    const targetFrame = Math.max(1, Math.min(this.frameCount, Math.round(clamped * (this.frameCount - 1)) + 1));
    if (targetFrame !== this.currentFrame) {
      this.currentFrame = targetFrame;
      this.render(targetFrame);
    }
  }

  private render(frameIdx: number): void {
    const img = this.images.get(frameIdx);
    if (!img || !this.ctx) return;
    this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
    this.ctx.drawImage(img, 0, 0, this.canvas.width, this.canvas.height);
  }
}
