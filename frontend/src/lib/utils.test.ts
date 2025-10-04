import { describe, it, expect } from 'vitest';

// Helper functions for formatting
export function formatPrice(price: number | null): string {
	if (!price) return 'N/A';
	return new Intl.NumberFormat('en-AU', {
		style: 'currency',
		currency: 'AUD',
		minimumFractionDigits: 0
	}).format(price);
}

export function formatYield(yieldVal: number | null): string {
	if (!yieldVal) return 'N/A';
	return `${yieldVal.toFixed(2)}%`;
}

describe('formatPrice', () => {
	it('should format valid price in AUD', () => {
		expect(formatPrice(650000)).toBe('$650,000');
	});

	it('should handle null price', () => {
		expect(formatPrice(null)).toBe('N/A');
	});

	it('should format small prices', () => {
		expect(formatPrice(1000)).toBe('$1,000');
	});

	it('should format large prices', () => {
		expect(formatPrice(1500000)).toBe('$1,500,000');
	});
});

describe('formatYield', () => {
	it('should format yield with 2 decimal places', () => {
		expect(formatYield(4.4)).toBe('4.40%');
	});

	it('should handle null yield', () => {
		expect(formatYield(null)).toBe('N/A');
	});

	it('should round yield correctly', () => {
		expect(formatYield(4.556)).toBe('4.56%');
	});

	it('should format low yields', () => {
		expect(formatYield(2.1)).toBe('2.10%');
	});

	it('should format high yields', () => {
		expect(formatYield(10.5)).toBe('10.50%');
	});
});