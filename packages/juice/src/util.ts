export function acceptString(obj: unknown) {
	return typeof obj === "string" ? obj : undefined;
}

export function acceptNumber(obj: unknown) {
	return typeof obj === "number" ? obj : undefined;
}

export function parseColor(color: unknown) {
	if (typeof color !== "string" || !color.startsWith("#")) {
		return undefined;
	}

	const hex = color.slice(1);
	const rgb = parseInt(hex, 16);

	if (Number.isNaN(rgb)) {
		return undefined;
	}

	if (hex.length === 6) {
		return [(rgb >> 16) & 0xff, (rgb >> 8) & 0xff, rgb & 0xff];
	} else if (hex.length === 3) {
		return [
			((rgb >> 8) & 0xf) * 0x11,
			((rgb >> 4) & 0xf) * 0x11,
			(rgb & 0xf) * 0x11,
		];
	} else {
		return undefined;
	}
}

export function parsePixels(value: unknown): number | undefined {
	if (typeof value === "number") {
		return value;
	} else if (typeof value === "string") {
		const match = value.match(/^(\d+)(px)?$/);
		if (match) return parseInt(match[1], 10);
	}
	return undefined;
}

export function acceptPixelsOrPercent(value: unknown): string | undefined {
	if (typeof value === "number") {
		return `${value}px`;
	} else if (typeof value === "string" && /^\d+(px|%)$/.test(value)) {
		return value;
	}else if (typeof value === "string" && /^\d+$/.test(value)) {
		return `${value}px`;
	} else {
		return undefined;
	}
}

export function calculateTRBL(
	all: unknown,
	x: unknown,
	y: unknown,
	top: unknown,
	right: unknown,
	bottom: unknown,
	left: unknown,
): [number, number, number, number] {
	const allPx = parsePixels(all);
	const xPx = parsePixels(x);
	const yPx = parsePixels(y);
	const topPx = parsePixels(top);
	const rightPx = parsePixels(right);
	const bottomPx = parsePixels(bottom);
	const leftPx = parsePixels(left);

	return [
		topPx ?? yPx ?? allPx ?? 0,
		rightPx ?? xPx ?? allPx ?? 0,
		bottomPx ?? yPx ?? allPx ?? 0,
		leftPx ?? xPx ?? allPx ?? 0,
	];
}
