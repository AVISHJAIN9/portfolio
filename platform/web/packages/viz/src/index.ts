export interface HistogramBucket {
  percentile: string;
  latencyNs: number;
  label: string;
}

export class LatencyChart {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D | null;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d");
  }

  public render(buckets: HistogramBucket[]): void {
    if (!this.ctx) return;
    const { width, height } = this.canvas;
    this.ctx.clearRect(0, 0, width, height);

    if (buckets.length === 0) return;

    const maxVal = Math.max(...buckets.map((b) => b.latencyNs), 1);
    const barWidth = (width - 40) / buckets.length;
    const padding = 8;

    buckets.forEach((b, i) => {
      const x = 20 + i * barWidth;
      const barHeight = (b.latencyNs / maxVal) * (height - 50);
      const y = height - 30 - barHeight;

      // Color by percentile severity
      let fill = "#10b981";
      if (b.percentile.includes("99")) fill = "#ef4444";
      else if (b.percentile.includes("90")) fill = "#f59e0b";
      else if (b.percentile.includes("75")) fill = "#38bdf8";

      this.ctx!.fillStyle = fill;
      this.ctx!.fillRect(x + padding / 2, y, barWidth - padding, barHeight);

      // Draw label
      this.ctx!.fillStyle = "#94a3b8";
      this.ctx!.font = "10px monospace";
      this.ctx!.textAlign = "center";
      this.ctx!.fillText(b.percentile, x + barWidth / 2, height - 12);
      this.ctx!.fillText(`${b.latencyNs}ns`, x + barWidth / 2, y - 5);
    });
  }
}

export class Sparkline {
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D | null;
  private history: number[] = [];
  private maxPoints: number;

  constructor(canvas: HTMLCanvasElement, maxPoints: number = 60) {
    this.canvas = canvas;
    this.ctx = canvas.getContext("2d");
    this.maxPoints = maxPoints;
  }

  public push(val: number): void {
    this.history.push(val);
    if (this.history.length > this.maxPoints) {
      this.history.shift();
    }
    this.draw();
  }

  private draw(): void {
    if (!this.ctx || this.history.length < 2) return;
    const { width, height } = this.canvas;
    this.ctx.clearRect(0, 0, width, height);

    const min = Math.min(...this.history);
    const max = Math.max(...this.history, min + 1);
    const stepX = width / (this.maxPoints - 1);

    this.ctx.beginPath();
    this.ctx.strokeStyle = "#38bdf8";
    this.ctx.lineWidth = 2;

    this.history.forEach((val, i) => {
      const x = i * stepX;
      const y = height - ((val - min) / (max - min)) * (height - 10) - 5;
      if (i === 0) this.ctx!.moveTo(x, y);
      else this.ctx!.lineTo(x, y);
    });
    this.ctx.stroke();
  }
}
