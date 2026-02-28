/** biome-ignore-all lint/suspicious/noExplicitAny: we're using well-documented preact internals */
import {
	type ComponentChild,
	type ContainerNode,
	options,
	render as preactRender,
} from "preact";
import { document } from "./UIDocument.js";
import { PressEvent } from "./UIEvent.js";
import "preact/hooks";

export type RendererEventCallback = (
	nodeId: number,
	event: { type: string; details: Record<string, unknown> },
) => void;

export interface UIRenderer {
	update(contents: string, eventCallback: RendererEventCallback): void;
}

declare global {
	const renderer: UIRenderer;
}

export function render(app: ComponentChild) {
	options.debounceRendering = (cb) => cb();

	const update = () => {
		const contents = JSON.stringify(document.firstChild);

		renderer.update(contents, (nodeId, event) => {
			const node = document.findElementByNodeId(nodeId);

			if (node) {
				node.dispatchEvent(
					new PressEvent(
						event.type,
						node,
						event.details as { x: number; y: number },
					),
				);
			}
		});
	};

	const commit = (options as any).__c;

	(options as any).__c = (vnode: any, commitQueue: any) => {
		commit?.(vnode, commitQueue);
		update();
	};

	preactRender(app, document as unknown as ContainerNode);
	update();
}
