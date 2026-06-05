# -*- coding: utf-8 -*-
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
import math


OUT_DIR = Path("assets")
OUT_DIR.mkdir(exist_ok=True)
OUT_FILE = OUT_DIR / "cheesebase_hybrid_flowchart.png"

W, H = 1500, 1100
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
F_BOX = make_font(26, True)
F_SMALL = make_font(20)
F_FORMULA = make_font(20)
F_TINY = make_font(17)

TEXT = "#263238"
MUTED = "#607080"
ORANGE = "#E67E22"
BLUE = "#2E86C1"
GREEN = "#27AE60"
PURPLE = "#8E44AD"
GOLD = "#B7950B"
RED = "#C0392B"
GRAY = "#95A5A6"
SHADOW = "#DDE4EE"
LINE = "#E7ECF3"


def text_size(text, font):
    box = d.textbbox((0, 0), text, font=font)
    return box[2] - box[0], box[3] - box[1]


def center_text(cx, cy, text, font, fill=TEXT, line_gap=7):
    lines = text.split("\n")
    heights = [text_size(line, font)[1] for line in lines]
    total_h = sum(heights) + line_gap * (len(lines) - 1)
    y = cy - total_h / 2
    for line, h in zip(lines, heights):
        w, _ = text_size(line, font)
        d.text((cx - w / 2, y), line, font=font, fill=fill)
        y += h + line_gap


def left_text(x, y, text, font, fill=TEXT, line_gap=8):
    for line in text.split("\n"):
        d.text((x, y), line, font=font, fill=fill)
        y += text_size(line, font)[1] + line_gap


def rounded_box(x, y, w, h, title, subtitle, fill, outline, index=None):
    d.rounded_rectangle((x + 6, y + 8, x + w + 6, y + h + 8), radius=22, fill=SHADOW)
    d.rounded_rectangle((x, y, x + w, y + h), radius=22, fill=fill, outline=outline, width=3)
    if index:
        d.text((x + 22, y + 18), index, font=make_font(29, True), fill=outline)
        title_cx = x + w / 2 + 15
    else:
        title_cx = x + w / 2
    center_text(title_cx, y + h / 2 - 12, title, F_BOX, TEXT, 5)
    if subtitle:
        center_text(x + w / 2, y + h - 28, subtitle, F_TINY, MUTED, 4)


def formula_box(x, y, w, h):
    d.rounded_rectangle((x + 6, y + 8, x + w + 6, y + h + 8), radius=22, fill=SHADOW)
    d.rounded_rectangle((x, y, x + w, y + h), radius=22, fill="#FFFDE7", outline=GOLD, width=3)
    center_text(x + w / 2, y + 34, "得分归一化与融合", F_BOX, TEXT)
    formula = (
        "bm25_norm = bm25_score / max_bm25_score\n"
        "vector_norm = clamp(vector_score, 0.0, 1.0)\n"
        "hybrid_score = 0.45 × bm25_norm\n"
        "             + 0.55 × vector_norm"
    )
    left_text(x + 34, y + 78, formula, F_FORMULA, "#37474F", 12)


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


def arrow(start, end, color, width=4):
    d.line((*start, *end), fill=color, width=width)
    draw_arrow_head(start, end, color)


def poly_arrow(points, color, width=4):
    for i in range(len(points) - 1):
        d.line((*points[i], *points[i + 1]), fill=color, width=width)
    draw_arrow_head(points[-2], points[-1], color)


# Title
center_text(W / 2, 58, "CheeseBase 向量检索与 Hybrid 融合流程图", F_TITLE, "#D35400")
center_text(W / 2, 108, "BM25 精确匹配 + Qdrant 语义召回 + 阈值过滤", F_SUB, MUTED)
d.line((120, 138, 1380, 138), fill=LINE, width=3)

# Input and two branches
rounded_box(590, 175, 320, 112, "本地索引", "index.json / 文档正文", "#EAF7EA", GREEN, "01")

rounded_box(170, 350, 310, 116, "BM25 检索", "关键词匹配与本地排序", "#F4ECF7", PURPLE, "02")
rounded_box(585, 350, 330, 116, "Chunk 切分", "约 700 字符，100 字符重叠", "#EAF4FF", BLUE, "03")
rounded_box(1030, 350, 310, 116, "Embedding", "DashScope 生成语义向量", "#EAF4FF", BLUE, "04")

rounded_box(1030, 545, 310, 116, "Qdrant 检索", "向量相似度召回片段", "#EAF4FF", BLUE, "05")
rounded_box(170, 545, 310, 116, "BM25 结果", "bm25_score", "#F4ECF7", PURPLE, "06")

formula_box(475, 650, 550, 220)

rounded_box(1080, 735, 330, 116, "阈值过滤", "HYBRID_SCORE_THRESHOLD = 0.45", "#FDEDEC", RED, "07")
rounded_box(585, 940, 330, 86, "最终搜索结果", "得分 / 文件名 / 页码 / 命中片段", "#FFFDE7", GOLD, "08")

# Arrows
poly_arrow([(750, 287), (750, 320), (325, 320), (325, 350)], GREEN)
arrow((750, 287), (750, 350), BLUE)
arrow((915, 408), (1030, 408), BLUE)
arrow((1185, 466), (1185, 545), BLUE)
arrow((325, 466), (325, 545), PURPLE)

poly_arrow([(325, 661), (325, 760), (475, 760)], PURPLE)
poly_arrow([(1185, 661), (1185, 760), (1025, 760)], BLUE)
arrow((1025, 793), (1080, 793), GOLD)
poly_arrow([(1245, 851), (1245, 983), (915, 983)], RED)
arrow((750, 870), (750, 940), GOLD)

# Explanatory labels
center_text(535, 318, "关键词通道", F_TINY, PURPLE)
center_text(995, 318, "语义通道", F_TINY, BLUE)
center_text(1158, 870, "过滤弱相关结果", F_TINY, RED)

# Footer
center_text(
    W / 2,
    1075,
    "说明：BM25 保证精确关键词召回，向量检索补充语义相似内容，Hybrid 融合后通过阈值控制结果质量。",
    F_TINY,
    "#6C7A89",
)

img.save(OUT_FILE, quality=95)
print(OUT_FILE.resolve())
