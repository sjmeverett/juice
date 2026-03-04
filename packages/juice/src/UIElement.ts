import type { ComponentChildren } from "preact";
import type { UIEventListener, UIEventMap } from "./UIEvent.js";
import { UINode } from "./UINode.js";
import { isUITextNodeJson } from "./UITextNode.js";
import {
  acceptNumber,
  acceptPixelsOrPercent,
  acceptString,
  calculateTRBL,
  parseColor,
  parsePixels,
} from "./util.js";

export type UIElementProps = {
  [K in keyof UIEventMap as `on${Capitalize<K>}`]?: UIEventListener<K>;
} & {
  children?: ComponentChildren;
};

let nextNodeId = 0;

export class UIElement<
  Props extends UIElementProps = UIElementProps,
> extends UINode {
  public static readonly namespace: string = "http://www.w3.org/1999/xhtml";

  public readonly nodeId;
  public readonly tagName: string;
  public readonly props: Partial<Props> = {};
  public style: Record<string, unknown> = {};

  constructor(tagName: string, namespaceURI: string | undefined = undefined) {
    super(UINode.ELEMENT_NODE, namespaceURI);
    this.tagName = tagName;
    this.nodeId = nextNodeId++;
  }

  setAttribute(key: string, val: unknown): void {
    (this.props as Record<string, unknown>)[key] = val;
  }

  removeAttribute(key: string): void {
    delete (this.props as Record<string, unknown>)[key];
  }

  toJSON(): unknown {
    const style = this.style;

    // Collapse adjacent text nodes into one so the renderer sees a
    // single text child instead of separate flex items.
    const children = this.childNodes.map((node) => node.toJSON());
    const collapsedChildren: unknown[] = [];

    for (const child of children) {
      const prev = collapsedChildren[collapsedChildren.length - 1];

      if (isUITextNodeJson(child) && isUITextNodeJson(prev)) {
        prev.text += child.text;
      } else {
        collapsedChildren.push(child);
      }
    }

    return {
      type: "element",
      tag:
        this.namespaceURI && this.namespaceURI !== UIElement.namespace
          ? `${this.namespaceURI}:${this.tagName}`
          : this.tagName,
      id: this.nodeId,
      children: collapsedChildren,

      alignItems: acceptString(style.alignItems),
      alignSelf: acceptString(style.alignSelf),
      background: parseColor(style.background),
      borderRadius: parsePixels(style.borderRadius),
      color: parseColor(style.color),
      flexBasis: parsePixels(style.flexBasis),
      flexDirection: acceptString(style.flexDirection),
      flexGrow: acceptNumber(style.flexGrow),
      flexShrink: acceptNumber(style.flexShrink),
      font: acceptString(style.font),
      fontSize: parsePixels(style.fontSize),
      gap: parsePixels(style.gap),
      height: acceptPixelsOrPercent(style.height),
      justifyContent: acceptString(style.justifyContent),
      justifySelf: acceptString(style.justifySelf),
      textAlign: acceptString(style.textAlign),
      width: acceptPixelsOrPercent(style.width),

      margin: calculateTRBL(
        style.margin,
        style.marginX,
        style.marginY,
        style.marginTop,
        style.marginRight,
        style.marginBottom,
        style.marginLeft,
      ),

      padding: calculateTRBL(
        style.padding,
        style.paddingX,
        style.paddingY,
        style.paddingTop,
        style.paddingRight,
        style.paddingBottom,
        style.paddingLeft,
      ),
    };
  }

  findElementByNodeId(id: number): UIElement | undefined {
    if (this.nodeId === id) {
      return this;
    }

    for (const child of this.childNodes) {
      if (!(child instanceof UIElement)) {
        continue;
      }

      const found = child.findElementByNodeId(id);

      if (found) {
        return found;
      }
    }

    return undefined;
  }
}
