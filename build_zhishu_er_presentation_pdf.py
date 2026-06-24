from __future__ import annotations

from pathlib import Path

from reportlab.lib import colors
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle, getSampleStyleSheet
from reportlab.lib.units import cm
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.platypus import Image, PageBreak, Paragraph, SimpleDocTemplate, Spacer, Table, TableStyle

from build_zhishu_closed_loop_docx import build_er_diagrams


OUTPUT = Path("知枢_企业知识资产管理与智能检索平台_执行手册_重排版版.pdf")
FONT_NAME = "MSYH"
FONT_BOLD = "MSYH-Bold"


def register_fonts() -> None:
    pdfmetrics.registerFont(TTFont(FONT_NAME, r"C:\Windows\Fonts\msyh.ttc"))
    pdfmetrics.registerFont(TTFont(FONT_BOLD, r"C:\Windows\Fonts\msyhbd.ttc"))


def build_styles():
    styles = getSampleStyleSheet()
    styles.add(
        ParagraphStyle(
            name="ZhTitle",
            parent=styles["Title"],
            fontName=FONT_BOLD,
            fontSize=24,
            leading=30,
            textColor=colors.HexColor("#1F4E79"),
            spaceAfter=12,
        )
    )
    styles.add(
        ParagraphStyle(
            name="ZhHeading",
            parent=styles["Heading2"],
            fontName=FONT_BOLD,
            fontSize=16,
            leading=22,
            textColor=colors.HexColor("#1F4E79"),
            spaceBefore=6,
            spaceAfter=8,
        )
    )
    styles.add(
        ParagraphStyle(
            name="ZhBody",
            parent=styles["BodyText"],
            fontName=FONT_NAME,
            fontSize=10.5,
            leading=17,
            textColor=colors.HexColor("#202020"),
            spaceAfter=6,
        )
    )
    styles.add(
        ParagraphStyle(
            name="ZhSmall",
            parent=styles["BodyText"],
            fontName=FONT_NAME,
            fontSize=9.5,
            leading=14,
            textColor=colors.HexColor("#5A5A5A"),
            spaceAfter=6,
        )
    )
    styles.add(
        ParagraphStyle(
            name="ZhBullet",
            parent=styles["BodyText"],
            fontName=FONT_NAME,
            fontSize=10.2,
            leading=16,
            leftIndent=14,
            bulletIndent=0,
            spaceAfter=5,
        )
    )
    return styles


def bullet(text: str, styles) -> Paragraph:
    return Paragraph(f"• {text}", styles["ZhBullet"])


def build_pdf() -> None:
    register_fonts()
    diagrams = build_er_diagrams()

    doc = SimpleDocTemplate(
        str(OUTPUT),
        pagesize=A4,
        leftMargin=1.0 * cm,
        rightMargin=1.0 * cm,
        topMargin=1.0 * cm,
        bottomMargin=1.0 * cm,
        title="Zhishu ER Diagrams",
        author="Codex",
    )

    story = [
        Image(str(diagrams["er1"]), width=19.0 * cm, height=11.2 * cm),
        PageBreak(),
        Image(str(diagrams["er2"]), width=19.0 * cm, height=11.2 * cm),
    ]

    doc.build(story)


if __name__ == "__main__":
    build_pdf()
