export type PdfLineStyle = 'body' | 'muted' | 'mono';

export interface PdfSectionLine {
	text: string;
	style?: PdfLineStyle;
}

export interface PdfSection {
	heading: string;
	lines: Array<string | PdfSectionLine>;
}

export interface StructuredPdfDefinition {
	fileName: string;
	title: string;
	subtitle?: string;
	metadata?: string[];
	sections: PdfSection[];
}

interface RenderStyle {
	font: 'F1' | 'F2' | 'F3';
	size: number;
	lineHeight: number;
	charLimit: number;
}

interface RenderLine {
	text: string;
	style: RenderStyle;
	y: number;
}

const PAGE_WIDTH = 612;
const PAGE_HEIGHT = 792;
const MARGIN_X = 54;
const MARGIN_TOP = 56;
const MARGIN_BOTTOM = 44;

const TITLE_STYLE: RenderStyle = {
	font: 'F2',
	size: 22,
	lineHeight: 30,
	charLimit: 48,
};

const SUBTITLE_STYLE: RenderStyle = {
	font: 'F1',
	size: 11,
	lineHeight: 16,
	charLimit: 82,
};

const HEADING_STYLE: RenderStyle = {
	font: 'F2',
	size: 14,
	lineHeight: 20,
	charLimit: 64,
};

const BODY_STYLE: RenderStyle = {
	font: 'F1',
	size: 10,
	lineHeight: 14,
	charLimit: 90,
};

const MUTED_STYLE: RenderStyle = {
	font: 'F1',
	size: 9,
	lineHeight: 13,
	charLimit: 92,
};

const MONO_STYLE: RenderStyle = {
	font: 'F3',
	size: 8,
	lineHeight: 11,
	charLimit: 80,
};

export function buildStructuredPdfDocument(definition: StructuredPdfDefinition): Uint8Array {
	const pages = paginate(buildRenderBlocks(definition));
	const pageObjectIds = pages.map((_, index) => 6 + index * 2);
	const contentObjectIds = pageObjectIds.map((id) => id + 1);
	const objects: string[] = [
		'<< /Type /Catalog /Pages 2 0 R >>',
		`<< /Type /Pages /Count ${pages.length} /Kids [${pageObjectIds.map((id) => `${id} 0 R`).join(' ')}] >>`,
		'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',
		'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >>',
		'<< /Type /Font /Subtype /Type1 /BaseFont /Courier >>',
	];

	for (let index = 0; index < pages.length; index += 1) {
		const pageObjectId = pageObjectIds[index];
		const contentObjectId = contentObjectIds[index];
		const content = buildPageContent(pages[index], index + 1, pages.length);
		const contentLength = encoder.encode(content).length;

		objects[pageObjectId - 1] =
			`<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${PAGE_WIDTH} ${PAGE_HEIGHT}] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >> >> /Contents ${contentObjectId} 0 R >>`;
		objects[contentObjectId - 1] =
			`<< /Length ${contentLength} >>\nstream\n${content}\nendstream`;
	}

	return serializePdf(objects);
}

export function downloadStructuredPdf(definition: StructuredPdfDefinition) {
	const payload = buildStructuredPdfDocument(definition);
	const bytes = new Uint8Array(payload.byteLength);
	bytes.set(payload);
	const blob = new Blob([bytes], { type: 'application/pdf' });
	const url = URL.createObjectURL(blob);
	const anchor = document.createElement('a');
	anchor.href = url;
	anchor.download = sanitizeFileName(definition.fileName);
	document.body.append(anchor);
	anchor.click();
	anchor.remove();
	window.setTimeout(() => URL.revokeObjectURL(url), 1_000);
}

export function buildTableLines(
	columns: string[],
	rows: Array<Array<unknown>>,
	maxRows = 10,
	maxColumns = 6,
): PdfSectionLine[] {
	const visibleColumns = columns.slice(0, maxColumns);
	const visibleRows = rows.slice(0, maxRows);
	const widths = visibleColumns.map((column, columnIndex) => {
		const values = visibleRows.map((row) => formatCell(row[columnIndex]));
		return Math.min(
			18,
			Math.max(
				sanitizeText(column).length,
				...values.map((value) => sanitizeText(value).length),
			),
		);
	});

	const renderCells = (values: string[]) =>
		values
			.map((value, index) => truncate(sanitizeText(value), widths[index]).padEnd(widths[index], ' '))
			.join(' | ')
			.trimEnd();

	const lines: PdfSectionLine[] = [];
	if (visibleColumns.length > 0) {
		lines.push({ text: renderCells(visibleColumns), style: 'mono' });
		lines.push({
			text: widths.map((width) => '-'.repeat(Math.max(3, Math.min(width, 12)))).join('-+-'),
			style: 'mono',
		});
	}

	for (const row of visibleRows) {
		lines.push({
			text: renderCells(visibleColumns.map((_, index) => formatCell(row[index]))),
			style: 'mono',
		});
	}

	if (columns.length > maxColumns) {
		lines.push({
			text: `Showing ${maxColumns} of ${columns.length} columns in the PDF snapshot.`,
			style: 'muted',
		});
	}

	if (rows.length > maxRows) {
		lines.push({
			text: `Showing ${maxRows} of ${rows.length} row(s) in the PDF snapshot.`,
			style: 'muted',
		});
	}

	return lines;
}

export function buildObjectTableLines(
	rows: Array<Record<string, unknown>>,
	maxRows = 10,
	maxColumns = 6,
): PdfSectionLine[] {
	const columns = Object.keys(rows[0] ?? {}).slice(0, maxColumns);
	return buildTableLines(
		columns,
		rows.slice(0, maxRows).map((row) => columns.map((column) => row[column])),
		maxRows,
		maxColumns,
	);
}

function buildRenderBlocks(definition: StructuredPdfDefinition) {
	const lines: Array<{ text: string; style: RenderStyle } | null> = [];

	lines.push({ text: definition.title, style: TITLE_STYLE });

	if (definition.subtitle) {
		lines.push({ text: definition.subtitle, style: SUBTITLE_STYLE });
	}

	for (const item of definition.metadata ?? []) {
		lines.push({ text: item, style: MUTED_STYLE });
	}

	lines.push(null);

	for (const section of definition.sections) {
		lines.push({ text: section.heading, style: HEADING_STYLE });
		for (const entry of section.lines) {
			if (typeof entry === 'string') {
				lines.push({ text: entry, style: BODY_STYLE });
				continue;
			}

			lines.push({
				text: entry.text,
				style: entry.style === 'mono' ? MONO_STYLE : entry.style === 'muted' ? MUTED_STYLE : BODY_STYLE,
			});
		}
		lines.push(null);
	}

	return lines;
}

function paginate(lines: Array<{ text: string; style: RenderStyle } | null>) {
	const pages: RenderLine[][] = [[]];
	let pageIndex = 0;
	let y = PAGE_HEIGHT - MARGIN_TOP;

	const nextPage = () => {
		pages.push([]);
		pageIndex += 1;
		y = PAGE_HEIGHT - MARGIN_TOP;
	};

	for (const line of lines) {
		if (line === null) {
			if (y - BODY_STYLE.lineHeight < MARGIN_BOTTOM) {
				nextPage();
			} else {
				y -= BODY_STYLE.lineHeight;
			}
			continue;
		}

		const wrapped = wrapText(line.text, line.style.charLimit);
		for (const fragment of wrapped) {
			if (y - line.style.lineHeight < MARGIN_BOTTOM) {
				nextPage();
			}

			pages[pageIndex].push({
				text: fragment,
				style: line.style,
				y,
			});
			y -= line.style.lineHeight;
		}
	}

	return pages.filter((page) => page.length > 0);
}

function buildPageContent(lines: RenderLine[], pageNumber: number, pageCount: number) {
	const commands = lines.map((line) =>
		`BT /${line.style.font} ${line.style.size} Tf 1 0 0 1 ${MARGIN_X} ${line.y} Tm (${escapePdfText(line.text)}) Tj ET`,
	);
	commands.push(
		`BT /F1 9 Tf 1 0 0 1 ${MARGIN_X} 24 Tm (${escapePdfText(`Page ${pageNumber} of ${pageCount}`)}) Tj ET`,
	);
	return commands.join('\n');
}

function serializePdf(objects: string[]) {
	let output = '%PDF-1.4\n';
	const offsets: number[] = [0];

	for (let index = 0; index < objects.length; index += 1) {
		offsets.push(encoder.encode(output).length);
		output += `${index + 1} 0 obj\n${objects[index]}\nendobj\n`;
	}

	const xrefOffset = encoder.encode(output).length;
	output += `xref\n0 ${objects.length + 1}\n`;
	output += '0000000000 65535 f \n';
	for (const offset of offsets.slice(1)) {
		output += `${String(offset).padStart(10, '0')} 00000 n \n`;
	}
	output += `trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xrefOffset}\n%%EOF`;
	return encoder.encode(output);
}

function wrapText(input: string, maxChars: number) {
	const text = sanitizeText(input);
	if (!text) {
		return [''];
	}

	const words = text.split(' ');
	const lines: string[] = [];
	let current = '';

	for (const word of words) {
		if (!current) {
			current = word;
			continue;
		}

		if (`${current} ${word}`.length <= maxChars) {
			current = `${current} ${word}`;
			continue;
		}

		lines.push(current);
		current = word;
	}

	if (current) {
		lines.push(current);
	}

	return lines.flatMap((line) => splitLongToken(line, maxChars));
}

function splitLongToken(text: string, maxChars: number) {
	if (text.length <= maxChars) {
		return [text];
	}

	const parts: string[] = [];
	for (let index = 0; index < text.length; index += maxChars) {
		parts.push(text.slice(index, index + maxChars));
	}
	return parts;
}

function formatCell(value: unknown) {
	if (value === null || value === undefined) {
		return '--';
	}

	if (typeof value === 'number' || typeof value === 'boolean') {
		return String(value);
	}

	if (typeof value === 'string') {
		return value;
	}

	try {
		return JSON.stringify(value);
	} catch {
		return String(value);
	}
}

function sanitizeText(value: string) {
	return value
		.normalize('NFKD')
		.replace(/[^\x20-\x7E]/g, '')
		.replace(/\s+/g, ' ')
		.trim();
}

function escapePdfText(value: string) {
	return sanitizeText(value).replace(/\\/g, '\\\\').replace(/\(/g, '\\(').replace(/\)/g, '\\)');
}

function sanitizeFileName(value: string) {
	const normalized = sanitizeText(value).replace(/[^a-zA-Z0-9._-]+/g, '-');
	if (normalized.toLowerCase().endsWith('.pdf')) {
		return normalized;
	}
	return `${normalized || 'export'}.pdf`;
}

function truncate(value: string, width: number) {
	if (value.length <= width) {
		return value;
	}
	if (width <= 3) {
		return value.slice(0, width);
	}
	return `${value.slice(0, width - 3)}...`;
}

const encoder = new TextEncoder();
