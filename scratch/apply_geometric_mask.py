from PIL import Image, ImageDraw

def mask_icon(input_path, output_path):
    # Load original image
    img = Image.open(input_path).convert("RGBA")
    w, h = img.size
    
    # Create mask image
    mask = Image.new("L", (w, h), 0)
    draw = ImageDraw.Draw(mask)
    
    # Define squircle boundaries
    # Based on blue bounds: x from 150 to 874, y from 150 to 874
    box = [(150, 150), (874, 874)]
    
    # Draw a rounded rectangle with corner radius 180
    # macOS squircle radius is about 180-200 for a 1024x1024 canvas (approx 17.5% of width)
    # 724 * 0.22 = 160 pixels
    radius = 165
    draw.rounded_rectangle(box, radius=radius, fill=255)
    
    # Apply mask as alpha channel
    img.putalpha(mask)
    
    # Save the result
    img.save(output_path, "PNG")
    print(f"Applied geometric squircle mask. Saved to {output_path}")

if __name__ == "__main__":
    import sys
    input_img = 'C:/Users/admin/.gemini/antigravity-ide/brain/2577cc2d-3237-40ce-a899-c94a5b388b3a/macos_whale_icon_1781945490198.png'
    mask_icon(input_img, 'desktop/src-tauri/icons/source-icon.png')
