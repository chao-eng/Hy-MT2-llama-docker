from PIL import Image
import numpy as np

img = Image.open('desktop/src-tauri/icons/source-icon.png')
img_np = np.array(img)
w, h = img.size

# Find pixels where the blue channel is dominant and not white/gray
# e.g., B - R > 30 and B > 50
blue_mask = (img_np[:, :, 2].astype(int) - img_np[:, :, 0].astype(int) > 30) & (img_np[:, :, 2] > 50)

y_indices, x_indices = np.where(blue_mask)
if len(y_indices) > 0:
    print(f"Blue region bounds: y min={y_indices.min()}, max={y_indices.max()} | x min={x_indices.min()}, max={x_indices.max()}")
else:
    print("No blue region found!")
