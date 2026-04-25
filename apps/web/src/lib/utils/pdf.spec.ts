import { describe, expect, it } from 'vitest';

import { buildStructuredPdfDocument, buildTableLines } from './pdf';

describe('pdf utils', () => {
	it('builds a valid pdf document payload', () => {
		const payload = buildStructuredPdfDocument({
			fileName: 'contour-export.pdf',
			title: 'Contour Export',
			subtitle: 'Dataset snapshot',
			metadata: ['Generated in test'],
			sections: [
				{
					heading: 'Primary analysis',
					lines: ['Row count: 12', { text: 'group | value', style: 'mono' }],
				},
			],
		});

		const text = new TextDecoder().decode(payload);
		expect(text.startsWith('%PDF-1.4')).toBe(true);
		expect(text).toContain('/Type /Catalog');
		expect(text).toContain('Contour Export');
		expect(text).toContain('startxref');
	});

	it('formats table snapshots with truncation hints', () => {
		const lines = buildTableLines(
			['group', 'value', 'count', 'region', 'owner', 'status', 'ignored'],
			[
				['Northwind', 1200, 18, 'EMEA', 'ops', 'healthy', 'foo'],
				['Lakehouse', 900, 12, 'US', 'sales', 'watch', 'bar'],
			],
			1,
			3,
		);

		expect(lines[0]).toMatchObject({ style: 'mono' });
		expect(lines.some((line) => line.style === 'muted' && line.text.includes('Showing 3 of 7 columns'))).toBe(true);
		expect(lines.some((line) => line.style === 'muted' && line.text.includes('Showing 1 of 2 row'))).toBe(true);
	});
});
