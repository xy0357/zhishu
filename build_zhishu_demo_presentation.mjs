import fs from "node:fs/promises";
import path from "node:path";
import { Presentation, PresentationFile } from "@oai/artifact-tool";

const projectRoot = process.env.PROJECT_ROOT || process.cwd();
const outputPath = process.env.OUTPUT_PPTX || path.join(projectRoot, "outputs", "zhishu_database_report.pptx");
const previewDir = process.env.PREVIEW_DIR || path.join(projectRoot, "outputs", "zhishu-demo-preview");
const contentPath = path.join(projectRoot, "ppt_content.json");
const slideSize = { width: 1280, height: 720 };

const palette = {
  paper: "#f5efe7",
  shell: "#fff9f1",
  white: "#ffffff",
  navy: "#12314d",
  blue: "#2f6fb9",
  blueSoft: "#d9ecff",
  teal: "#1f8a8a",
  tealSoft: "#dff5f5",
  gold: "#c38d3b",
  goldSoft: "#f8ecd7",
  coral: "#d46a6a",
  coralSoft: "#fbe7e7",
  ink: "#1f2f3d",
  subInk: "#5d7287",
  border: "#cad7e4",
  line: "#a3bdd3",
  codeBg: "#eef4fb",
  lavender: "#ebe3fb",
  violet: "#7d59b5",
  mint: "#dff1e4",
  green: "#2e7d55"
};

const titleFont = "STXingkai";
const bodyFont = "STKaiti";
const bodyStrongFont = "SimKai";
const codeFont = "Consolas";

function panelAccent(index) {
  const accents = [
    { line: palette.blue, fill: palette.blueSoft },
    { line: palette.teal, fill: palette.tealSoft },
    { line: palette.gold, fill: palette.goldSoft },
    { line: palette.violet, fill: palette.lavender },
    { line: palette.coral, fill: palette.coralSoft },
    { line: palette.green, fill: palette.mint }
  ];
  return accents[index % accents.length];
}

function createPresentation() {
  return Presentation.create({ slideSize });
}

function addText(slide, config) {
  const shape = slide.shapes.add({
    geometry: "textbox",
    position: config.position,
    fill: "none",
    line: { style: "solid", fill: "none", width: 0 }
  });
  shape.text = config.text;
  shape.text.style = {
    fontSize: config.fontSize,
    color: config.color || palette.ink,
    bold: config.bold || false,
    fontFace: config.fontFace || bodyFont,
    alignment: config.alignment || "left",
    valign: config.valign || "top",
    italic: config.italic || false
  };
  return shape;
}

function addCard(slide, position, options = {}) {
  return slide.shapes.add({
    geometry: "roundRect",
    position,
    fill: options.fill || palette.white,
    line: {
      style: "solid",
      fill: options.lineFill || palette.border,
      width: options.lineWidth || 1
    },
    borderRadius: options.borderRadius || "rounded-2xl",
    shadow: options.shadow || "shadow-sm"
  });
}

function addPill(slide, text, position, fill, color) {
  const pill = slide.shapes.add({
    geometry: "roundRect",
    position,
    fill,
    line: { style: "solid", fill, width: 0 },
    borderRadius: "rounded-full"
  });
  pill.text = text;
  pill.text.style = {
    fontSize: 16,
    color: color || palette.ink,
    bold: true,
    fontFace: bodyStrongFont,
    alignment: "center",
    valign: "middle"
  };
}

function addSectionHeader(slide, slideNo, section, title, subtitle) {
  addPill(slide, String(slideNo).padStart(2, "0"), { left: 70, top: 36, width: 70, height: 28 }, palette.blueSoft, palette.blue);
  addText(slide, {
    position: { left: 152, top: 34, width: 240, height: 28 },
    text: section,
    fontSize: 16,
    bold: true,
    fontFace: bodyStrongFont,
    color: palette.subInk
  });
  addText(slide, {
    position: { left: 74, top: 84, width: 760, height: 52 },
    text: title,
    fontSize: 36,
    bold: true,
    fontFace: bodyStrongFont,
    color: palette.navy
  });
  if (subtitle) {
    addText(slide, {
      position: { left: 76, top: 138, width: 1120, height: 40 },
      text: subtitle,
      fontSize: 20,
      fontFace: bodyFont,
      color: palette.subInk
    });
  }
}

function addDivider(slide, left, top, width) {
  slide.shapes.add({
    geometry: "rect",
    position: { left, top, width, height: 2 },
    fill: palette.line,
    line: { style: "solid", fill: palette.line, width: 0 }
  });
}

function addBulletBlock(slide, title, bullets, position, accent, bodySize = 18) {
  addCard(slide, position, { fill: palette.white, lineFill: accent.line });
  addCard(slide, { left: position.left, top: position.top, width: position.width, height: 8 }, { fill: accent.line, lineFill: accent.line, lineWidth: 0, borderRadius: "rounded-2xl" });
  addText(slide, {
    position: { left: position.left + 20, top: position.top + 18, width: position.width - 40, height: 32 },
    text: title,
    fontSize: 23,
    bold: true,
    fontFace: bodyStrongFont,
    color: accent.line
  });
  addText(slide, {
    position: { left: position.left + 20, top: position.top + 58, width: position.width - 40, height: position.height - 72 },
    text: bullets.map((item) => `\u2022 ${item}`).join("\n"),
    fontSize: bodySize,
    fontFace: bodyFont,
    color: palette.ink
  });
}

function addMetricCard(slide, title, value, subtitle, position, accent) {
  addCard(slide, position, { fill: accent.fill, lineFill: accent.fill, lineWidth: 0 });
  addText(slide, {
    position: { left: position.left + 22, top: position.top + 18, width: position.width - 44, height: 24 },
    text: title,
    fontSize: 18,
    fontFace: bodyStrongFont,
    color: palette.subInk
  });
  addText(slide, {
    position: { left: position.left + 22, top: position.top + 48, width: position.width - 44, height: 42 },
    text: value,
    fontSize: 32,
    bold: true,
    fontFace: bodyStrongFont,
    color: accent.line
  });
  addText(slide, {
    position: { left: position.left + 22, top: position.top + 94, width: position.width - 44, height: 54 },
    text: subtitle,
    fontSize: 16,
    fontFace: bodyFont,
    color: palette.ink
  });
}

function addCodePanel(slide, title, text, position) {
  addCard(slide, position, { fill: palette.codeBg, lineFill: palette.border });
  addText(slide, {
    position: { left: position.left + 16, top: position.top + 12, width: position.width - 32, height: 24 },
    text: title,
    fontSize: 18,
    bold: true,
    fontFace: bodyStrongFont,
    color: palette.navy
  });
  addText(slide, {
    position: { left: position.left + 16, top: position.top + 42, width: position.width - 32, height: position.height - 56 },
    text,
    fontSize: 14,
    fontFace: codeFont,
    color: palette.ink
  });
}

async function readImageBlob(imagePath) {
  const bytes = await fs.readFile(imagePath);
  return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
}

async function addImage(slide, imagePath, position, options = {}) {
  const contentType = imagePath.toLowerCase().endsWith(".png") ? "image/png" : "image/jpeg";
  return slide.images.add({
    blob: await readImageBlob(imagePath),
    contentType,
    alt: options.alt || path.basename(imagePath),
    fit: options.fit || "cover",
    position,
    geometry: options.geometry || "roundRect",
    borderRadius: options.borderRadius || "rounded-2xl"
  });
}

async function loadJson() {
  return JSON.parse(await fs.readFile(contentPath, "utf8"));
}

async function loadSql() {
  return fs.readFile(path.join(projectRoot, "backend", "migrations", "001_init.sql"), "utf8");
}

async function loadSource(filePath) {
  return fs.readFile(path.join(projectRoot, filePath), "utf8");
}

function extractTableSnippet(sql, tableName, maxLines = 12) {
  const pattern = new RegExp(`CREATE TABLE IF NOT EXISTS ${tableName} \\(([\\s\\S]*?)\\n\\);`, "m");
  const match = sql.match(pattern);
  if (!match) return `${tableName} table not found`;
  const lines = match[0].split("\n").slice(0, maxLines);
  if (match[0].split("\n").length > maxLines) lines.push("  ...");
  return lines.join("\n");
}

function extractFunctionBlock(source, signature, lineCount = 28) {
  const lines = source.split("\n");
  const index = lines.findIndex((line) => line.includes(signature));
  if (index < 0) return `${signature} not found`;
  return lines.slice(index, index + lineCount).join("\n");
}

function extractQuestionInsertBlock(source) {
  const anchor = 'let insert_question = sqlx::query(';
  return extractFunctionBlock(source, anchor, 24);
}

function extractCitationInsertBlock(source) {
  const anchor = 'INSERT INTO answer_citations (';
  return extractFunctionBlock(source, anchor, 20);
}

function extractAgentInsertBlock(source) {
  const anchor = 'INSERT INTO agent_runs (';
  return extractFunctionBlock(source, anchor, 18);
}

function extractForeignKeys(sql, tableName) {
  const pattern = new RegExp(`CREATE TABLE IF NOT EXISTS ${tableName} \\(([\\s\\S]*?)\\n\\);`, "m");
  const match = sql.match(pattern);
  if (!match) return [];
  return match[1]
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.startsWith("CONSTRAINT fk_"));
}

function countTables(sql) {
  return (sql.match(/CREATE TABLE IF NOT EXISTS/g) || []).length;
}

async function buildDeck() {
  const content = await loadJson();
  const sql = await loadSql();
  const documentRoutes = await loadSource("backend/src/routes/documents.rs");
  const qaRoutes = await loadSource("backend/src/routes/qa.rs");
  const mysqlStore = await loadSource("backend/src/store/mysql.rs");
  const totalTables = countTables(sql);

  const shots = {
    dashboard: path.join(projectRoot, "tmp", "ppt_video_frames_full", "dashboard.png"),
    qa: path.join(projectRoot, "tmp", "ppt_video_frames_full", "qa.png"),
    agent: path.join(projectRoot, "tmp", "ppt_video_frames_full", "agent.png"),
    er1: path.join(projectRoot, "tmp", "er_diagrams", "er-1-core-knowledge.png"),
    er2: path.join(projectRoot, "tmp", "er_diagrams", "er-2-qa-traceability.png")
  };

  const deck = createPresentation();

  {
    const slide = deck.slides.add();
    slide.background.fill = palette.paper;
    addCard(slide, { left: 46, top: 30, width: 1188, height: 658 }, { fill: palette.shell, lineFill: palette.goldSoft });
    addText(slide, {
      position: { left: 90, top: 78, width: 520, height: 76 },
      text: content.deck_title,
      fontSize: 60,
      bold: true,
      fontFace: titleFont,
      color: palette.navy
    });
    addText(slide, {
      position: { left: 92, top: 154, width: 520, height: 100 },
      text: content.deck_subtitle,
      fontSize: 40,
      bold: true,
      fontFace: bodyStrongFont,
      color: palette.ink
    });
    addText(slide, {
      position: { left: 92, top: 282, width: 520, height: 130 },
      text: content.deck_desc,
      fontSize: 24,
      fontFace: bodyFont,
      color: palette.subInk
    });
    addPill(slide, content.cover_tags[0], { left: 92, top: 444, width: 152, height: 40 }, palette.goldSoft, palette.gold);
    addPill(slide, content.cover_tags[1], { left: 260, top: 444, width: 280, height: 40 }, palette.blueSoft, palette.blue);
    await addImage(slide, shots.dashboard, { left: 670, top: 78, width: 500, height: 286 }, { alt: "dashboard" });
    await addImage(slide, shots.qa, { left: 670, top: 388, width: 500, height: 222 }, { alt: "qa" });
  }

  {
    const data = content.slides.outline;
    const slide = deck.slides.add();
    slide.background.fill = palette.paper;
    addSectionHeader(slide, 2, data.section, data.title, data.subtitle);
    addCard(slide, { left: 86, top: 198, width: 1108, height: 388 }, { fill: palette.white, lineFill: palette.goldSoft });
    data.items.forEach((item, idx) => {
      const top = 226 + idx * 52;
      addPill(slide, String(idx + 1).padStart(2, "0"), { left: 118, top, width: 68, height: 30 }, panelAccent(idx).fill, panelAccent(idx).line);
      addText(slide, {
        position: { left: 208, top: top - 2, width: 860, height: 30 },
        text: item,
        fontSize: 24,
        bold: idx < 2,
        fontFace: idx < 2 ? bodyStrongFont : bodyFont,
        color: palette.ink
      });
      addDivider(slide, 118, top + 38, 1016);
    });
  }

  {
    const data = content.slides.requirements;
    const slide = deck.slides.add();
    slide.background.fill = palette.paper;
    addSectionHeader(slide, 3, data.section, data.title, data.subtitle);
    data.blocks.forEach((block, idx) => {
      addBulletBlock(slide, block.title, block.bullets, { left: 78 + idx * 378, top: 188, width: idx === 2 ? 366 : 350, height: 246 }, panelAccent(idx), 19);
    });
    addCard(slide, { left: 82, top: 468, width: 1114, height: 116 }, { fill: palette.shell, lineFill: palette.goldSoft });
    addText(slide, {
      position: { left: 108, top: 494, width: 1060, height: 66 },
      text: data.summary,
      fontSize: 25,
      bold: true,
      fontFace: bodyStrongFont,
      color: palette.ink
    });
  }

  {
    const data = content.slides.goals;
    const slide = deck.slides.add();
    slide.background.fill = palette.paper;
    addSectionHeader(slide, 4, data.section, data.title, data.subtitle);
    data.metrics.forEach((item, idx) => {
      addMetricCard(slide, item[0], item[1], item[2], { left: 84 + idx * 272, top: 188, width: idx === 3 ? 298 : 248, height: 156 }, panelAccent(idx));
    });
    addBulletBlock(slide, "\u65b9\u6848\u8bf4\u660e", data.bullets, { left: 84, top: 386, width: 1112, height: 176 }, panelAccent(4), 20);
  }

  {
    const data = content.slides.architecture;
    const slide = deck.slides.add();
    slide.background.fill = palette.paper;
    addSectionHeader(slide, 5, data.section, data.title, data.subtitle);
    data.layers.forEach((item, idx) => {
      const widths = [244, 244, 244, 204];
      const lefts = [82, 386, 690, 994];
      addBulletBlock(slide, item[0], item[1], { left: lefts[idx], top: 194, width: widths[idx], height: 206 }, panelAccent(idx), 18);
      if (idx < data.layers.length - 1) {
        addText(slide, {
          position: { left: lefts[idx] + widths[idx] + 10, top: 268, width: 40, height: 40 },
          text: "\u2192",
          fontSize: 32,
          bold: true,
          fontFace: bodyStrongFont,
          color: palette.gold,
          alignment: "center"
        });
      }
    });
    addCard(slide, { left: 84, top: 430, width: 1112, height: 140 }, { fill: palette.shell, lineFill: palette.goldSoft });
    addText(slide, {
      position: { left: 108, top: 456, width: 1060, height: 88 },
      text: data.summary,
      fontSize: 24,
      fontFace: bodyFont,
      color: palette.ink
    });
  }

  {
    const data = content.slides.er1;
    const slide = deck.slides.add();
    slide.background.fill = palette.paper;
    addSectionHeader(slide, 6, data.section, data.title, data.subtitle);
    await addImage(slide, shots.er1, { left: 74, top: 188, width: 472, height: 330 }, { alt: "er1", fit: "contain" });
    addBulletBlock(slide, "\u5173\u7cfb\u89e3\u8bfb", data.bullets_a, { left: 582, top: 188, width: 614, height: 208 }, panelAccent(1), 18);
    addBulletBlock(slide, "\u6c47\u62a5\u8981\u70b9", data.bullets_b, { left: 582, top: 420, width: 614, height: 152 }, panelAccent(0), 18);
  }

  {
    const data = content.slides.er2;
    const slide = deck.slides.add();
    slide.background.fill = palette.paper;
    addSectionHeader(slide, 7, data.section, data.title, data.subtitle);
    await addImage(slide, shots.er2, { left: 78, top: 188, width: 472, height: 326 }, { alt: "er2", fit: "contain" });
    addBulletBlock(slide, "\u8ffd\u6eaf\u903b\u8f91", data.bullets_a, { left: 584, top: 188, width: 612, height: 184 }, panelAccent(2), 18);
    addBulletBlock(slide, "\u5ba1\u8ba1\u4ef7\u503c", data.bullets_b, { left: 584, top: 392, width: 612, height: 176 }, panelAccent(3), 18);
  }

  {
    const data = content.slides.schema;
    const slide = deck.slides.add();
    slide.background.fill = palette.paper;
    addSectionHeader(slide, 8, data.section, data.title, data.subtitle.replace('16', String(totalTables)));
    data.groups.forEach((group, idx) => {
      const widths = [252, 252, 252, 294];
      const lefts = [82, 352, 622, 892];
      addBulletBlock(slide, group[0], group[1], { left: lefts[idx], top: 190, width: widths[idx], height: 224 }, panelAccent(idx), 18);
    });
    addCard(slide, { left: 84, top: 444, width: 1112, height: 122 }, { fill: palette.shell, lineFill: palette.goldSoft });
    addText(slide, {
      position: { left: 108, top: 470, width: 1060, height: 76 },
      text: data.summary,
      fontSize: 24,
      fontFace: bodyFont,
      color: palette.ink
    });
  }

  {
    const data = content.slides.frontend;
    const slide = deck.slides.add();
    slide.background.fill = palette.paper;
    addSectionHeader(slide, 9, data.section, data.title, data.subtitle);
    await addImage(slide, shots.dashboard, { left: 82, top: 188, width: 350, height: 196 }, { alt: "dashboard" });
    await addImage(slide, shots.qa, { left: 462, top: 188, width: 350, height: 196 }, { alt: "qa" });
    await addImage(slide, shots.agent, { left: 842, top: 188, width: 350, height: 196 }, { alt: "agent" });
    data.cards.forEach((card, idx) => {
      addBulletBlock(slide, card[0], card[1], { left: 82 + idx * 380, top: 408, width: 350, height: 164 }, panelAccent(idx), 18);
    });
  }

  {
    const data = content.slides.backend_api;
    const slide = deck.slides.add();
    slide.background.fill = palette.paper;
    addSectionHeader(slide, 10, data.section, data.title, data.subtitle);
    addCodePanel(slide, "documents.rs", extractFunctionBlock(documentRoutes, "pub async fn create_document(", 28), { left: 72, top: 190, width: 540, height: 330 });
    addCodePanel(slide, "qa.rs", extractFunctionBlock(qaRoutes, "pub async fn ask_question(", 20), { left: 668, top: 190, width: 540, height: 330 });
    addBulletBlock(slide, "CRUD \u5bf9\u5e94\u5173\u7cfb", data.bullets, { left: 72, top: 540, width: 1136, height: 116 }, panelAccent(4), 18);
  }

  {
    const data = content.slides.backend_sql;
    const slide = deck.slides.add();
    slide.background.fill = palette.paper;
    addSectionHeader(slide, 11, data.section, data.title, data.subtitle);
    addCodePanel(slide, "questions / answers", extractQuestionInsertBlock(mysqlStore), { left: 72, top: 188, width: 372, height: 336 });
    addCodePanel(slide, "answer_citations", extractCitationInsertBlock(mysqlStore), { left: 454, top: 188, width: 372, height: 336 });
    addCodePanel(slide, "agent_runs", extractAgentInsertBlock(mysqlStore), { left: 836, top: 188, width: 372, height: 336 });
    addBulletBlock(slide, "\u6570\u636e\u5e93\u8bf4\u660e", data.bullets, { left: 72, top: 544, width: 1136, height: 112 }, panelAccent(1), 18);
  }

  {
    const data = content.slides.highlights;
    const slide = deck.slides.add();
    slide.background.fill = palette.paper;
    addSectionHeader(slide, 12, data.section, data.title, data.subtitle);
    data.blocks.forEach((block, idx) => {
      addBulletBlock(slide, block[0], block[1], { left: 84 + idx * 372, top: 194, width: idx === 2 ? 360 : 344, height: 188 }, panelAccent(idx), 19);
    });
    const fkSummary = [
      `documents \u542b ${extractForeignKeys(sql, "documents").length} \u4e2a\u5916\u952e`,
      `document_versions \u542b ${extractForeignKeys(sql, "document_versions").length} \u4e2a\u5916\u952e`,
      `answer_citations \u542b ${extractForeignKeys(sql, "answer_citations").length} \u4e2a\u5916\u952e`,
      `agent_runs \u542b ${extractForeignKeys(sql, "agent_runs").length} \u4e2a\u5916\u952e`
    ];
    addBulletBlock(slide, "\u7ed3\u6784\u5316\u7ea6\u675f", [
      "\u901a\u8fc7\u4e3b\u952e\u3001\u5916\u952e\u548c\u4e2d\u95f4\u8868\u628a\u6838\u5fc3\u5b9e\u4f53\u5173\u7cfb\u843d\u5230\u6570\u636e\u5e93\u4e2d\u3002",
      `\u5916\u952e\u5206\u5e03\uff1a${fkSummary.join("\uFF1B")}`,
      "\u6570\u636e\u5e93\u5c42\u9762\u7684\u8bbe\u8ba1\u76f4\u63a5\u652f\u6491\u4e0a\u5c42\u68c0\u7d22\u3001\u95ee\u7b54\u4e0e\u5ba1\u8ba1\u80fd\u529b\u3002"
    ], { left: 84, top: 420, width: 1116, height: 150 }, panelAccent(5), 18);
    addCard(slide, { left: 84, top: 586, width: 1116, height: 68 }, { fill: palette.shell, lineFill: palette.goldSoft });
    addText(slide, {
      position: { left: 110, top: 606, width: 1060, height: 28 },
      text: data.summary,
      fontSize: 22,
      bold: true,
      fontFace: bodyStrongFont,
      color: palette.ink
    });
  }

  {
    const data = content.slides.summary;
    const slide = deck.slides.add();
    slide.background.fill = palette.paper;
    addSectionHeader(slide, 13, data.section, data.title, data.subtitle);
    data.blocks.forEach((block, idx) => {
      addBulletBlock(slide, block[0], block[1], { left: 84 + idx * 380, top: 202, width: idx === 1 ? 360 : 340, height: 176 }, panelAccent(idx), 19);
    });
    addCard(slide, { left: 86, top: 420, width: 1110, height: 164 }, { fill: palette.shell, lineFill: palette.goldSoft });
    addText(slide, {
      position: { left: 116, top: 450, width: 1044, height: 110 },
      text: "\u77e5\u67a2\u9879\u76ee\u628a\u9700\u6c42\u5206\u6790\u3001E-R \u56fe\u3001\u5173\u7cfb\u6a21\u5f0f\u3001SQL \u843d\u5e93\u4e0e\u667a\u80fd\u95ee\u7b54\u6269\u5c55\u6574\u5408\u6210\u4e86\u4e00\u4e2a\u53ef\u8fd0\u884c\u3001\u53ef\u89e3\u91ca\u3001\u53ef\u5c55\u793a\u7684\u6570\u636e\u5e93\u7cfb\u7edf\u8bfe\u7a0b\u5b9e\u8df5\u6210\u679c\u3002",
      fontSize: 28,
      bold: true,
      fontFace: bodyStrongFont,
      color: palette.ink
    });
  }

  await fs.mkdir(previewDir, { recursive: true });
  for (const [index, slide] of deck.slides.items.entries()) {
    const png = await deck.export({ slide, format: "png", scale: 1 });
    const filePath = path.join(previewDir, `slide-${String(index + 1).padStart(2, "0")}.png`);
    await fs.writeFile(filePath, new Uint8Array(await png.arrayBuffer()));
  }

  const montage = await deck.export({ format: "webp", montage: true, scale: 1 });
  await fs.writeFile(path.join(previewDir, "deck-montage.webp"), new Uint8Array(await montage.arrayBuffer()));

  await fs.mkdir(path.dirname(outputPath), { recursive: true });
  const pptx = await PresentationFile.exportPptx(deck);
  await pptx.save(outputPath);
  console.log(outputPath);
}

buildDeck().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
