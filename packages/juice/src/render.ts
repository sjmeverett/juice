/** biome-ignore-all lint/suspicious/noExplicitAny: we're using well-documented preact internals */
import {
  type ComponentChild,
  type ContainerNode,
  options,
  render as preactRender,
} from "preact";
import { document } from "./JuiceDocument.js";
import { JuiceEvent } from "./JuiceEvent.js";
import "preact/hooks";

export type RendererEventCallback = (
  nodeId: number,
  event: { type: string; details: Record<string, unknown> },
) => void;

export interface JuiceRenderer {
  update(eventCallback: RendererEventCallback): void;
  addFont(name: string, contents: string): void;
}

declare global {
  const renderer: JuiceRenderer;
}

export function render(app: ComponentChild) {
  options.debounceRendering = (cb) => cb();

  const update = () => {
    // console.log(JSON.stringify(document.documentElement, null, 2));

    renderer.update((nodeId, event) => {
      const node = document.documentElement.findElementByNodeId(nodeId);

      if (node) {
        node.dispatchEvent(new JuiceEvent(event.type, node, event.details));
      } else {
        console.error(
          `Attempt to dispatch ${event.type} to non-existent node ${nodeId}`,
        );
      }
    });
  };

  const prevUnmount = (options as any).unmount;
  (options as any).unmount = (vnode: any) => {
    prevUnmount?.(vnode);
    // Only delete DOM nodes owned by this vnode (not component vnodes
    // which borrow __e from their first child)
    const node = vnode.__e;
    if (node?.nodeId != null && typeof vnode.type !== "function") {
      dom.deleteNode(node.nodeId);
    }
  };

  const commit = (options as any).__c;
  (options as any).__c = (vnode: any, commitQueue: any) => {
    commit?.(vnode, commitQueue);
    update();
  };

  preactRender(app, document as unknown as ContainerNode);
  update();
}
