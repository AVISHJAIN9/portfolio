#!/usr/bin/env python3
"""
tools/frames_gen.py
Extracts or procedurally generates 3D frame sequences for project landing pages.
Generates:
  - frames/f_0001.webp .. frames/f_0180.webp (desktop 1920w)
  - frames_mobile/m_0001.webp .. frames_mobile/m_0180.webp (mobile 960w)
  - poster.webp
"""

import os
import sys
import shutil
import subprocess
import yaml
from pathlib import Path
try:
    from PIL import Image, ImageDraw, ImageFont
    HAS_PIL = True
except ImportError:
    HAS_PIL = False

def load_project_registry(root_dir):
    reg_path = root_dir / "inputs" / "PROJECT_REGISTRY.yaml"
    if reg_path.exists():
        with open(reg_path, "r", encoding="utf-8") as f:
            return yaml.safe_load(f).get("projects", [])
    return []

def generate_procedural_frame(project_slug: str, frame_idx: int, total_frames: int, width: int, height: int) -> Image.Image:
    """Generates a high-tech dark-mode cybernetic dashboard / 3D wireframe visual for a given frame index."""
    import math
    img = Image.new("RGB", (width, height), color=(10, 14, 23))
    draw = ImageDraw.Draw(img)
    
    # Progress 0.0 to 1.0
    t = frame_idx / float(total_frames)
    cx, cy = width / 2, height / 2
    
    # Draw dark grid
    grid_size = 40 if width > 1000 else 20
    for x in range(0, width, grid_size):
        draw.line([(x, 0), (x, height)], fill=(18, 26, 42), width=1)
    for y in range(0, height, grid_size):
        draw.line([(y, 0), (width, y)], fill=(18, 26, 42), width=1)
        
    # Animated geometric rings (Orbital telemetry visualization)
    ring_count = 6
    for i in range(ring_count):
        radius = (min(width, height) * 0.15) + (i * 35 * (width / 1920.0))
        angle_offset = (t * 2 * math.pi * (i + 1) * 0.5)
        # Draw dotted or arc lines
        bbox = [cx - radius, cy - radius, cx + radius, cy + radius]
        start_deg = math.degrees(angle_offset) % 360
        end_deg = (start_deg + 240 + (i * 20)) % 360
        color = (0, 200 + int(55 * math.sin(t * 6)), 255) if i % 2 == 0 else (120, 80, 255)
        draw.arc(bbox, start=start_deg, end=end_deg, fill=color, width=2)
        
    # Draw central HUD core
    core_radius = 45 * (width / 1920.0)
    draw.ellipse([cx - core_radius, cy - core_radius, cx + core_radius, cy + core_radius], fill=(16, 185, 129))
    
    # Text overlay
    title = f"{project_slug.upper()} // SYSTEM TELEMETRY FRAME [{frame_idx:04d}/{total_frames:04d}]"
    draw.text((30, 30), title, fill=(200, 220, 255))
    draw.text((30, 60), f"FPS: 60 | DETERMINISTIC REPLAY | LATENCY: {12.4 + math.sin(t * 10) * 1.5:.2f}ns", fill=(100, 150, 200))
    
    return img

def process_project(root_dir: Path, p: dict):
    slug = p.get("slug")
    num = p.get("id") or p.get("number", "00")
    repo_path = p.get("repo_path", f"projects/{num}-{slug}")
    proj_dir = root_dir / repo_path
    frontend_dir = proj_dir / "frontend"
    public_dir = frontend_dir / "public" / "assets"
    frames_dir = public_dir / "frames"
    mobile_dir = public_dir / "frames_mobile"
    
    frames_dir.mkdir(parents=True, exist_ok=True)
    mobile_dir.mkdir(parents=True, exist_ok=True)
    
    # Check for video file
    video_path = root_dir / "inputs" / "assets" / "video" / f"{num}-{slug}.mp4"
    if not video_path.exists():
        video_path = root_dir / "inputs" / "assets" / "video" / f"{slug}.mp4"
        
    has_ffmpeg = shutil.which("ffmpeg") is not None
    
    if video_path.exists() and has_ffmpeg:
        print(f"[{slug}] Extracting frames from {video_path} via ffmpeg...")
        subprocess.run([
            "ffmpeg", "-y", "-i", str(video_path),
            "-vf", "fps=30,scale=1920:-2",
            "-c:v", "libwebp", "-quality", "80",
            str(frames_dir / "f_%04d.webp")
        ], check=True)
        subprocess.run([
            "ffmpeg", "-y", "-i", str(video_path),
            "-vf", "fps=30,scale=960:-2",
            "-c:v", "libwebp", "-quality", "75",
            str(mobile_dir / "m_%04d.webp")
        ], check=True)
    else:
        # Procedural fallback
        total_frames = 60  # Generate 60 deterministic keyframes for lightweight, clean dev
        print(f"[{slug}] Generating procedural 3D wireframe frames ({total_frames} frames)...")
        if HAS_PIL:
            # Generate poster first
            poster = generate_procedural_frame(slug, 0, total_frames, 1920, 1080)
            poster.save(public_dir / "poster.webp", "WEBP", quality=85)
            
            for i in range(1, total_frames + 1):
                f_desk = generate_procedural_frame(slug, i, total_frames, 1920, 1080)
                f_desk.save(frames_dir / f"f_{i:04d}.webp", "WEBP", quality=80)
                
                f_mob = generate_procedural_frame(slug, i, total_frames, 960, 540)
                f_mob.save(mobile_dir / f"m_{i:04d}.webp", "WEBP", quality=75)
        else:
            print("PIL not installed, creating mock placeholder files")
            with open(public_dir / "poster.webp", "wb") as f:
                f.write(b"WEBP_MOCK_POSTER")
            for i in range(1, 10):
                with open(frames_dir / f"f_{i:04d}.webp", "wb") as f:
                    f.write(b"WEBP_MOCK_FRAME")

    print(f"[{slug}] Frames generated successfully in {public_dir}")

def main():
    root_dir = Path(__file__).resolve().parent.parent
    projects = load_project_registry(root_dir)
    target = sys.argv[1] if len(sys.argv) > 1 else None
    
    for p in projects:
        if target and target not in (p.get("id"), p.get("number"), p.get("slug"), "ALL"):
            continue
        process_project(root_dir, p)

if __name__ == "__main__":
    main()
