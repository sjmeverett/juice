import { JuiceElement } from "./JuiceElement.js";
import { PressEvent } from "./JuiceEvent.js";
import { JuiceImgElement } from "./JuiceImgElement.js";
import JuiceLayoutElement from "./JuiceLayoutElement.js";
import type { JuiceNode } from "./JuiceNode.js";
import { JuiceSvgElement } from "./JuiceSvgElement.js";
import { JuiceTextNode } from "./JuiceTextNode.js";

export class JuiceDocument extends JuiceLayoutElement {
  constructor() {
    super("document");

    let pressedNode: JuiceNode | undefined;

    this.addEventListener("PressIn", (event) => {
      pressedNode = event.target;
    });

    this.addEventListener("PressOut", (event) => {
      if (pressedNode?.contains(event.target)) {
        pressedNode.dispatchEvent(
          new PressEvent(
            "Press",
            pressedNode,
            event.details as { x: number; y: number },
          ),
        );
      }

      pressedNode = undefined;
    });
  }

  createElement(tag: string): JuiceElement {
    return this.createElementNS(JuiceElement.namespace, tag);
  }

  createElementNS(namespaceURI: string, tagName: string): JuiceElement {
    if (namespaceURI === JuiceSvgElement.namespace) {
      if (tagName === "svg") {
        return new JuiceSvgElement();
      } else {
        return new JuiceElement(tagName);
      }
    } else if (tagName === "img") {
      return new JuiceImgElement();
    } else {
      return new JuiceLayoutElement(tagName);
    }
  }

  createTextNode(text: string): JuiceTextNode {
    return new JuiceTextNode(text);
  }

  get documentElement() {
    return this;
  }
}

export const document = new JuiceDocument();
(globalThis as unknown as { document: JuiceDocument }).document = document;

declare global {
  interface Dom {
    createElement(tag: string): number;
    createTextNode(text: string): number;
    appendChild(parentId: number, childId: number): void;
    insertChildAt(index: number, parentId: number, childId: number): void;
    removeChild(parentId: number, childId: number): void;
    deleteNode(nodeId: number): void;
    setAttributeString(nodeId: number, key: string, value: string): void;
    setAttributeNumber(nodeId: number, key: string, value: number): void;
    setStyleString(nodeId: number, key: string, value: string): void;
    setStyleNumber(nodeId: number, key: string, value: number): void;
    setStylePercent(nodeId: number, key: string, value: number): void;
    setStyleEm(nodeId: number, key: string, value: number): void;
  }

  const dom: Dom;
}
