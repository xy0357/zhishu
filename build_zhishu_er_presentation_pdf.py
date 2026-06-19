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
    styles = build_styles()
    diagrams = build_er_diagrams()

    doc = SimpleDocTemplate(
        str(OUTPUT),
        pagesize=A4,
        leftMargin=1.5 * cm,
        rightMargin=1.5 * cm,
        topMargin=1.6 * cm,
        bottomMargin=1.4 * cm,
        title="知枢 ER 图与关系模式宣讲版",
        author="Codex",
    )

    story = []
    story.append(Paragraph("知枢", styles["ZhTitle"]))
    story.append(Paragraph("企业知识资产管理与智能检索平台", styles["ZhHeading"]))
    story.append(Paragraph("ER 图与关系模式宣讲版", styles["ZhHeading"]))
    story.append(
        Paragraph(
            "本版 PDF 按数据库课程标准答题格式整理：先画实体，再画属性和联系，在线两端标注 1 或 n；再把 E-R 图转换为中文关系模式。",
            styles["ZhBody"],
        )
    )
    story.append(Spacer(1, 0.2 * cm))
    story.append(bullet("ER-1 先讲基础知识管理域，重点是角色、用户、分类、文档、标签、文档版本。", styles))
    story.append(bullet("ER-2 再讲问答与版本追溯域，重点是问题、回答、引用证据和文档版本之间的关系。", styles))
    story.append(bullet("关系模式严格按“先写实体，再并入 1:1 / 1:n 联系，最后单独写 n:n 联系”的规则转换。", styles))
    story.append(Spacer(1, 0.3 * cm))

    split_table = Table(
        [
            ["子图", "覆盖实体", "讲解重点"],
            ["ER-1 基础知识管理域", "角色 / 用户 / 分类 / 文档 / 标签 / 文档版本", "先讲实体，再逐条读联系线两端的 1 和 n"],
            ["ER-2 问答与版本追溯域", "用户 / 问题 / 回答 / 引用证据 / 文档 / 文档版本", "强调回答通过引用证据回溯到具体文档版本"],
        ],
        colWidths=[3.5 * cm, 7.2 * cm, 6.0 * cm],
    )
    split_table.setStyle(
        TableStyle(
            [
                ("FONTNAME", (0, 0), (-1, 0), FONT_BOLD),
                ("FONTNAME", (0, 1), (-1, -1), FONT_NAME),
                ("FONTSIZE", (0, 0), (-1, -1), 9.2),
                ("LEADING", (0, 0), (-1, -1), 12),
                ("BACKGROUND", (0, 0), (-1, 0), colors.HexColor("#4F81BD")),
                ("TEXTCOLOR", (0, 0), (-1, 0), colors.white),
                ("BACKGROUND", (0, 1), (-1, -1), colors.HexColor("#F7FAFC")),
                ("ROWBACKGROUNDS", (0, 1), (-1, -1), [colors.HexColor("#EDF3F9"), colors.white]),
                ("GRID", (0, 0), (-1, -1), 0.6, colors.HexColor("#9FB7CC")),
                ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
                ("LEFTPADDING", (0, 0), (-1, -1), 6),
                ("RIGHTPADDING", (0, 0), (-1, -1), 6),
                ("TOPPADDING", (0, 0), (-1, -1), 5),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 5),
            ]
        )
    )
    story.append(split_table)
    story.append(PageBreak())

    figure_payloads = [
        (
            "ER-1 基础知识管理域",
            diagrams["er1"],
            [
                "这一张图只保留最核心的知识管理实体，不再把阅读记录、收藏记录这些实现层行为画成概念实体。",
                "角色到用户是 1:n，说明一个角色可以对应多个用户；用户到文档是 1:n，说明一个用户可以创建多篇文档。",
                "分类到文档是 1:n，说明分类承担稳定归档作用；文档到标签是 n:n，说明标签承担灵活标注作用。",
                "文档到文档版本是 1:n，说明一个文档会形成多个历史版本，这是知识追溯的基础。",
            ],
        ),
        (
            "ER-2 问答与版本追溯域",
            diagrams["er2"],
            [
                "这一张图只保留问答闭环里真正需要讲清楚的 6 个核心对象，不再把片段、文件、Agent 运行这些实现层对象放进概念图。",
                "用户到问题是 1:n，问题到回答是 1:n，表示一个用户可以提多个问题，一个问题也可能生成多次回答。",
                "回答到引用证据是 1:n，这是关键设计，因为一个回答往往需要多条证据支撑。",
                "引用证据再定位到文档版本，表示答案依据的是某个具体版本，而不是一篇抽象文档。",
            ],
        ),
    ]

    for title, image_path, notes in figure_payloads:
        story.append(Paragraph(title, styles["ZhHeading"]))
        story.append(Image(str(image_path), width=17.3 * cm, height=10.2 * cm))
        story.append(Spacer(1, 0.15 * cm))
        for note in notes:
            story.append(bullet(note, styles))
        story.append(PageBreak())

    story.append(Paragraph("关系模式怎么讲", styles["ZhHeading"]))
    story.append(
        Paragraph(
            "讲关系模式时，直接按数据库课程标准规则展开：先写实体，再把 1:1 和 1:n 联系并入 n 端实体，外码不是主属性；最后再写 n:n 联系转换出的新关系模式。",
            styles["ZhBody"],
        )
    )
    chinese_mode_table = Table(
        [
            ["中文关系模式", "说明"],
            ["角色（角色编号，角色名称，角色说明）", "独立实体"],
            ["用户（用户编号，用户名，部门，角色编号(FK)）", "1:n 联系“拥有”并入用户"],
            ["分类（分类编号，分类名称，分类说明）", "独立实体"],
            ["文档（文档编号，标题，状态，分类编号(FK)，创建者编号(FK)）", "“归属”“创建”联系并入文档"],
            ["标签（标签编号，标签名称，标签说明）", "独立实体"],
            ["文档版本（版本编号，版本号，变更说明，文档编号(FK)）", "“形成版本”联系并入文档版本"],
            ["问题（问题编号，问题内容，状态，用户编号(FK)）", "“提出”联系并入问题"],
            ["回答（回答编号，模型，回答时间，问题编号(FK)）", "“生成回答”联系并入回答"],
            ["引用证据（引用编号，证据顺序，回答编号(FK)，版本编号(FK)）", "“引用”“定位版本”联系并入引用证据"],
            ["文档标签（文档编号(FK)，标签编号(FK)）", "n:n 联系单独转换；联合主码可取（文档编号，标签编号）"],
        ],
        colWidths=[11.0 * cm, 7.0 * cm],
    )
    chinese_mode_table.setStyle(
        TableStyle(
            [
                ("FONTNAME", (0, 0), (-1, 0), FONT_BOLD),
                ("FONTNAME", (0, 1), (-1, -1), FONT_NAME),
                ("FONTSIZE", (0, 0), (-1, -1), 8.8),
                ("LEADING", (0, 0), (-1, -1), 11),
                ("BACKGROUND", (0, 0), (-1, 0), colors.HexColor("#4F81BD")),
                ("TEXTCOLOR", (0, 0), (-1, 0), colors.white),
                ("ROWBACKGROUNDS", (0, 1), (-1, -1), [colors.HexColor("#EDF3F9"), colors.white]),
                ("GRID", (0, 0), (-1, -1), 0.6, colors.HexColor("#9FB7CC")),
                ("VALIGN", (0, 0), (-1, -1), "TOP"),
                ("LEFTPADDING", (0, 0), (-1, -1), 6),
                ("RIGHTPADDING", (0, 0), (-1, -1), 6),
                ("TOPPADDING", (0, 0), (-1, -1), 5),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 5),
            ]
        )
    )
    story.append(chinese_mode_table)
    story.append(Spacer(1, 0.25 * cm))
    relationship_table = Table(
        [
            ["核心结构", "你要强调的点", "一句话讲法"],
            ["documents + document_versions", "当前快照与历史版本分离", "主表负责当前展示，版本表负责历史追溯。"],
            ["answers + answer_citations", "答案与证据解耦", "一个回答可以对应多条证据，不再局限单文档引用。"],
            ["documents + tags + document_tags", "主分类与灵活标签分离", "分类负责稳定归档，标签负责灵活标注。"],
            ["users + questions + answers", "问答责任链清晰", "谁提问、问题如何生成回答，都可以明确追溯。"],
        ],
        colWidths=[5.0 * cm, 5.3 * cm, 6.0 * cm],
    )
    relationship_table.setStyle(
        TableStyle(
            [
                ("FONTNAME", (0, 0), (-1, 0), FONT_BOLD),
                ("FONTNAME", (0, 1), (-1, -1), FONT_NAME),
                ("FONTSIZE", (0, 0), (-1, -1), 9.4),
                ("LEADING", (0, 0), (-1, -1), 12),
                ("BACKGROUND", (0, 0), (-1, 0), colors.HexColor("#4F81BD")),
                ("TEXTCOLOR", (0, 0), (-1, 0), colors.white),
                ("ROWBACKGROUNDS", (0, 1), (-1, -1), [colors.HexColor("#EDF3F9"), colors.white]),
                ("GRID", (0, 0), (-1, -1), 0.6, colors.HexColor("#9FB7CC")),
                ("VALIGN", (0, 0), (-1, -1), "MIDDLE"),
                ("LEFTPADDING", (0, 0), (-1, -1), 6),
                ("RIGHTPADDING", (0, 0), (-1, -1), 6),
                ("TOPPADDING", (0, 0), (-1, -1), 6),
                ("BOTTOMPADDING", (0, 0), (-1, -1), 6),
            ]
        )
    )
    story.append(relationship_table)
    story.append(Spacer(1, 0.3 * cm))
    story.append(Paragraph("老师常见追问的简答模板", styles["ZhHeading"]))
    story.append(bullet("为什么属性里不写外码：因为外码属于关系模式层，不属于概念 E-R 图里的普通属性。", styles))
    story.append(bullet("为什么不把阅读记录、收藏记录画成核心实体：因为它们更像实现层行为表，不是概念层主对象。", styles))
    story.append(bullet("为什么文档和标签要单独转成文档标签关系模式：因为它们是 n:n 联系。", styles))
    story.append(bullet("为什么回答不直接连文档：因为一个回答往往依赖多条证据，所以要先经过引用证据。", styles))
    story.append(Spacer(1, 0.3 * cm))
    story.append(
        Paragraph(
            "宣讲顺序建议：业务目标 → 先讲实体 → 再讲联系和基数 → 再把 E-R 图翻译成中文关系模式。这样最符合数据库课程设计的标准答题口径。",
            styles["ZhBody"],
        )
    )

    doc.build(story)


if __name__ == "__main__":
    build_pdf()
