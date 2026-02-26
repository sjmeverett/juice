import type { ComponentChildren } from "preact";

export class UIEvent<
	T extends Record<string, unknown> = Record<never, unknown>,
> {
	public readonly type: string;
	public readonly target: UINode;
	public readonly details: T;
	private _propagationStopped: boolean;

	constructor(type: string, target: UINode, details: T) {
		this.type = type;
		this.target = target;
		this.details = details;
		this._propagationStopped = false;
	}

	stopPropagation() {
		this._propagationStopped = true;
	}

	get propagationStopped(): boolean {
		return this._propagationStopped;
	}
}

export class PressEvent extends UIEvent<{ x: number; y: number }> {}

export interface UIEventMap {
	PressIn: PressEvent;
	PressOut: PressEvent;
	Press: PressEvent;
	PressMove: PressEvent;
}

export type UIEventListener<Event extends keyof UIEventMap> = (
	event: UIEventMap[Event],
) => void;

export class UINode {
	public readonly nodeType: number;
	public readonly namespaceURI: string | null;
	public readonly childNodes: UINode[];

	public id?: number;
	public parentNode: UINode | null;
	private _eventListeners: Map<string, Set<Function>> = new Map();

	public static ELEMENT_NODE = 1;
	public static TEXT_NODE = 3;

	constructor(nodeType: number, namespaceURI: string | null = null) {
		this.nodeType = nodeType;
		this.namespaceURI = namespaceURI;
		this.parentNode = null;
		this.childNodes = [];
	}

	get firstChild() {
		return this.childNodes[0] ?? null;
	}

	get nextSibling(): UINode | null {
		if (!this.parentNode) return null;

		const siblings = this.parentNode.childNodes;
		return siblings[siblings.indexOf(this) + 1] ?? null;
	}

	contains(other: UINode | null): boolean {
		if (other === null) return false;
		if (other === this) return true;

		return this.childNodes.some(
			(child) => child === other || child.contains(other),
		);
	}

	insertBefore(node: UINode, child: UINode | null): UINode {
		node.parentNode = this;
		const idx = child ? this.childNodes.indexOf(child) : -1;

		if (idx >= 0) {
			this.childNodes.splice(idx, 0, node);
		} else {
			this.childNodes.push(node);
		}

		return node;
	}

	addEventListener<E extends keyof UIEventMap>(
		type: E,
		listener: UIEventListener<E>,
	): () => void {
		let set = this._eventListeners.get(type);
		if (!set) {
			set = new Set();
			this._eventListeners.set(type, set);
		}
		set.add(listener);
		return () => set!.delete(listener);
	}

	removeEventListener<E extends keyof UIEventMap>(
		type: E,
		listener: UIEventListener<E>,
	): void {
		this._eventListeners.get(type)?.delete(listener);
	}

	dispatchEvent(event: UIEvent) {
		const listeners = this._eventListeners.get(event.type);
		if (listeners) {
			for (const fn of listeners) {
				fn.call(this, event);
			}
		}

		if (!event.propagationStopped) {
			this.parentNode?.dispatchEvent(event);
		}
	}

	appendChild(node: UINode): UINode {
		node.parentNode = this;
		this.childNodes.push(node);
		return node;
	}

	removeChild(child: UINode): UINode {
		const idx = this.childNodes.indexOf(child);

		if (idx >= 0) {
			this.childNodes.splice(idx, 1);
		}

		child.parentNode = null;
		return child;
	}

	prepend(node: UINode): UINode {
		node.parentNode = this;
		this.childNodes.unshift(node);
		return node;
	}
}

export type UIElementProps = {
	[K in keyof UIEventMap as `on${Capitalize<K>}`]?: UIEventListener<K>;
} & {
	children?: ComponentChildren;
};

export class UIElement<
	Props extends UIElementProps = UIElementProps,
> extends UINode {
	public readonly tagName: string;
	public readonly props: Partial<Props> = {};
	public style: Record<string, unknown> = {};

	constructor(tagName: string, namespaceURI: string | null = null) {
		super(UINode.ELEMENT_NODE, namespaceURI);
		this.tagName = tagName;
	}

	setAttribute(key: string, val: unknown): void {
		(this.props as Record<string, unknown>)[key] = val;
	}

	removeAttribute(key: string): void {
		delete (this.props as Record<string, unknown>)[key];
	}
}

export class UITextNode extends UINode {
	private _textContent = "";

	constructor(text: string) {
		super(UINode.TEXT_NODE, null);
		this._textContent = text;
	}

	get data(): string {
		return this._textContent;
	}

	set data(text: string) {
		this._textContent = text;
	}

	get textContent(): string {
		return this._textContent;
	}

	set textContent(text: string) {
		this._textContent = text;
	}
}

interface UIDocument {
	createElement(tagName: string): UIElement;
	createElementNS(namespaceURI: string, tagName: string): UIElement;
	createTextNode(text: string): UITextNode;
}

const document = (globalThis as unknown as { document: UIDocument }).document;

document.createElement = (tagName: string) => new UIElement(tagName);

document.createElementNS = (namespaceURI: string, tagName: string) =>
	new UIElement(tagName, namespaceURI);

document.createTextNode = (text: string) => new UITextNode(text);

export type RendererEventCallback = (
	nodeId: number,
	event: { type: string; details: Record<string, unknown> },
) => void;

export interface UIRenderer {
	update(contents: string, eventCallback: RendererEventCallback): void;
}

declare global {
	const renderer: UIRenderer;
	function nativeLog(message: string): void;
}
