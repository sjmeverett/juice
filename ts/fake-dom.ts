type FakeNode = NativeNode | NativeTextNode;

export class NativeNode {
	nodeType = 1;
	type: string;
	props: Record<string, unknown> = {};
	style: Record<string, string> = {};
	children: FakeNode[] = [];
	parentNode: NativeNode | null = null;
	_listeners?: Record<string, EventListener>;
	_id?: number;

	constructor(type: string) {
		this.type = type;
	}

	appendChild(child: FakeNode): FakeNode {
		child.parentNode = this;
		this.children.push(child);
		return child;
	}

	insertBefore(child: FakeNode, ref: FakeNode | null): FakeNode {
		child.parentNode = this;
		const idx = ref ? this.children.indexOf(ref) : -1;
		if (idx >= 0) this.children.splice(idx, 0, child);
		else this.children.push(child);
		return child;
	}

	removeChild(child: FakeNode): FakeNode {
		const idx = this.children.indexOf(child);
		if (idx >= 0) this.children.splice(idx, 1);
		child.parentNode = null;
		return child;
	}

	setAttribute(key: string, val: unknown): void {
		this.props[key] = val;
	}

	removeAttribute(key: string): void {
		delete this.props[key];
	}

	addEventListener(event: string, handler: EventListener): void {
		if (!this._listeners) this._listeners = {};
		this._listeners[event] = handler;
	}

	removeEventListener(event: string): void {
		if (this._listeners) delete this._listeners[event];
	}

	dispatchEvent(event: { type: string }): void {
		const handler = this._listeners?.[event.type];
		if (handler) handler.call(this, event as Event);
	}

	get firstChild(): FakeNode | null {
		return this.children[0] || null;
	}

	get nextSibling(): FakeNode | null {
		if (!this.parentNode) return null;
		const siblings = this.parentNode.children;
		return siblings[siblings.indexOf(this) + 1] || null;
	}
}

export class NativeTextNode {
	nodeType = 3;
	data: string;
	parentNode: NativeNode | null = null;

	constructor(text: string) {
		this.data = text;
	}

	get nextSibling(): FakeNode | null {
		if (!this.parentNode) return null;
		const siblings = this.parentNode.children;
		return siblings[siblings.indexOf(this) + 1] || null;
	}
}

// Augment globalThis with our bridge functions
declare global {
	var __TREE__: string;
	var __refreshTree__: () => void;
	var __dispatchEvent__: (nodeId: number, eventType: string) => void;
	// Native functions registered by Rust host
	function nativeLog(message: string): void;
}

// Global "document" that Preact will use
(globalThis as any).document = {
	createElement(tag: string) {
		return new NativeNode(tag);
	},
	createElementNS(_ns: string, tag: string) {
		return new NativeNode(tag);
	},
	createTextNode(text: string) {
		return new NativeTextNode(text);
	},
};
