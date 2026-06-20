from PIL import Image
import numpy as np
from collections import deque
import sys

def process_icon(input_path, output_path):
    img = Image.open(input_path).convert("RGBA")
    w, h = img.size
    img_np = np.array(img)
    
    # Extract RGB channels
    rgb = img_np[:, :, :3]
    
    # Calculate saturation
    sat = np.max(rgb, axis=2).astype(int) - np.min(rgb, axis=2).astype(int)
    
    # The generated image background is pure white or very close to it.
    # So we'll check for low saturation (sat <= 10) and high brightness (mean_val > 240)
    mean_val = np.mean(rgb, axis=2)
    is_bg_candidate = (sat <= 10) & (mean_val > 240)
    
    visited = np.zeros((h, w), dtype=bool)
    queue = deque()
    
    # Add border pixels to queue
    for x in range(w):
        if is_bg_candidate[0, x]:
            queue.append((0, x))
            visited[0, x] = True
        if is_bg_candidate[h-1, x]:
            queue.append((h-1, x))
            visited[h-1, x] = True
            
    for y in range(h):
        if is_bg_candidate[y, 0]:
            queue.append((y, 0))
            visited[y, 0] = True
        if is_bg_candidate[y, w-1]:
            queue.append((y, w-1))
            visited[y, w-1] = True
            
    # BFS
    directions = [(-1, 0), (1, 0), (0, -1), (0, 1)]
    while queue:
        cy, cx = queue.popleft()
        for dy, dx in directions:
            ny, nx = cy + dy, cx + dx
            if 0 <= ny < h and 0 <= nx < w:
                if not visited[ny, nx] and is_bg_candidate[ny, nx]:
                    visited[ny, nx] = True
                    queue.append((ny, nx))
                    
    # Make visited transparent
    img_np[visited, 3] = 0
    
    # Save directly to output
    out_img = Image.fromarray(img_np)
    out_img.save(output_path, "PNG")
    print(f"Processed new icon and saved to {output_path}")
    print(f"Transparent pixels: {np.sum(visited)} out of {w * h} ({np.sum(visited)/(w*h)*100:.1f}%)")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python process_new_icon.py <input_path>")
        sys.exit(1)
    process_icon(sys.argv[1], 'desktop/src-tauri/icons/source-icon.png')
