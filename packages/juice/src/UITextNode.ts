import { UINode } from "./UINode.js";

export class UITextNode extends UINode {
	private _textContent = "";

	constructor(text: string) {
		super(UINode.TEXT_NODE, null);
		this._textContent = text;
	}

	// preact expects it to be called "data" for some reason
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

	toJSON() {
		return { type: "text", text: String(this._textContent) };
	}
}
