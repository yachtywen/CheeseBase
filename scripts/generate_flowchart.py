# -*- coding: utf-8 -*-
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
import math


OUT_DIR = Path("assets")
OUT_DIR.mkdir(exist_ok=True)
OUT_FILE = OUT_DIR / "cheesebase_flowchart.png"

W, H = 1400, 1050
img = Image.new("RGB", (W, H), "#FAFBFD")
d = ImageDraw.Draw(img)


def make_font(size: int, bold: bool = False):
    bold_candidates = [
        r"C:\Windows\Fonts\msyhbd.ttc",
        r"C:\Windows\Fonts\simhei.ttf",
    ]
    normal_candidates = [
        r"C:\Windows\Fonts\msyh.ttc",
        r"C:\Windows\Fonts\simhei.ttf",
        r"C:\Windows\Fonts\simsun.ttc",
    ]
    candidates = bold_candidates + normal_candidates if bold else normal_candidates
    for path in candidates:
        if Path(path).exists():
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


F_TITLE = make_font(44, True)
F_SUB = make_font(23)
F_BOX = make_font(27, True)
F_SMALL = make_font(20)
F_TINY = make_font(18)

TEXT = "#263238"
MUTED = "#607080"
ORANGE = "#E67E22"
BLUE = "#2E86C1"
GREEN = "#27AE60"
PURPLE = "#8E44AD"
GOLD = "#B7950B"
GRAY = "#95A5A6"
SHADOW = "#DDE4EE"
LINE = "#E7ECF3"


def text_size(text, font):
    box = d.textbbox((0, 0), text, font=font)
    return box[2] - box[0], box[3] - box[1]


def center_text(cx, cy, text, font, fill=TEXT, line_gap=8):
    lines = text.split("\n")
    heights = [text_size(line, font)[1] for line in lines]
    total_h = sum(heights) + line_gap * (len(lines) - 1)
    y = cy - total_h / 2
    for line, h in zip(lines, heights):
        w, _ = text_size(line, font)
        d.text((cx - w / 2, y), line, font=font, fill=fill)
        y += h + line_gap


def rounded_box(x, y, w, h, title, subtitle, fill, outline, index):
    d.rounded_rectangle((x + 6, y + 8, x + w + 6, y + h + 8), radius=22, fill=SHADOW)
    d.rounded_rectangle((x, y, x + w, y + h), radius=22, fill=fill, outline=outline, width=3)
    d.text((x + 24, y + 20), index, font=make_font(30, True), fill=outline)
    center_text(x + w / 2 + 18, y + h / 2 - 10, title, F_BOX, TEXT, 5)
    center_text(x + w / 2, y + h - 28, subtitle, F_TINY, MUTED, 4)


def arrow(start, end, color, width=4):
    x1, y1 = start
    x2, y2 = end
    d.line((x1, y1, x2, y2), fill=color, width=width)
    draw_arrow_head(start, end, color)


def draw_arrow_head(start, end, color, size=16):
    x1, y1 = start
    x2, y2 = end
    angle = math.atan2(y2 - y1, x2 - x1)
    a1 = angle + math.pi * 0.82
    a2 = angle - math.pi * 0.82
    points = [
        (x2, y2),
        (x2 + size * math.cos(a1), y2 + size * math.sin(a1)),
        (x2 + size * math.cos(a2), y2 + size * math.sin(a2)),
    ]
    d.polygon(points, fill=color)


def dashed_polyline(points, color=GRAY, width=3, dash=16, gap=10):
    for i in range(len(points) - 1):
        x1, y1 = points[i]
        x2, y2 = points[i + 1]
        dx, dy = x2 - x1, y2 - y1
        dist = math.hypot(dx, dy)
        if dist == 0:
            continue
        ux, uy = dx / dist, dy / dist
        t = 0
        while t < dist:
            a = t
            b = min(t + dash, dist)
            d.line((x1 + ux * a, y1 + uy * a, x1 + ux * b, y1 + uy * b), fill=color, width=width)
            t += dash + gap
    draw_arrow_head(points[-2], points[-1], color, 14)


# Title
center_text(W / 2, 58, "CheeseBase 本地知识库混合检索流程图", F_TITLE, "#D35400")
center_text(W / 2, 108, "Rust + BM25 + Qdrant：从本地文件到可视化检索结果", F_SUB, MUTED)
d.line((120, 138, 1280, 138), fill=LINE, width=3)

# Boxes: square-ish layout
rounded_box(570, 175, 260, 110, "知识库目录", "knowledge_base", "#FFF3E0", ORANGE, "01")

rounded_box(110, 345, 280, 118, "文档扫描", "Scanner / 多级目录", "#EAF4FF", BLUE, "02")
rounded_box(560, 345, 280, 118, "内容解析", "文本 / 代码 / PDF", "#EAF4FF", BLUE, "03")
rounded_box(1010, 345, 280, 118, "分词处理", "Tokenizer / 中英文", "#EAF4FF", BLUE, "04")

rounded_box(560, 530, 280, 118, "本地索引", "倒排索引 + index.json", "#EAF7EA", GREEN, "05")

rounded_box(255, 705, 300, 120, "BM25 检索", "关键词匹配与排序", "#F4ECF7", PURPLE, "06")
rounded_box(845, 705, 300, 120, "向量索引", "Embedding + Qdrant", "#F4ECF7", PURPLE, "07")

rounded_box(560, 875, 280, 118, "Hybrid 融合", "加权排序 + 阈值过滤", "#FFFDE7", GOLD, "08")
rounded_box(990, 875, 310, 118, "CLI / TUI 展示", "搜索 / 预览 / 跳转", "#FFFDE7", GOLD, "09")

# Main arrows
arrow((700, 285), (250, 345), ORANGE)
arrow((390, 404), (560, 404), BLUE)
arrow((840, 404), (1010, 404), BLUE)
arrow((1150, 463), (840, 589), GREEN)
arrow((700, 648), (405, 705), PURPLE)
arrow((700, 648), (995, 705), PURPLE)
arrow((405, 825), (560, 934), GOLD)
arrow((995, 825), (840, 934), GOLD)
arrow((840, 934), (990, 934), GOLD)

# Update loop, routed around the chart.
dashed_polyline([(1300, 934), (1340, 934), (1340, 660), (170, 660), (170, 463)], GRAY, 3)
center_text(345, 638, "/update 重新扫描并刷新索引", F_TINY, GRAY)

# Footer note
center_text(
    W / 2,
    1023,
    "说明：BM25 为默认本地检索；Hybrid 模式结合 BM25 与 Qdrant 向量检索，适合语义搜索。",
    F_TINY,
    "#6C7A89",
)

img.save(OUT_FILE, quality=95)
print(OUT_FILE.resolve())
