import type { ComponentChildren } from "preact";
import type { UIEventListener, UIEventMap } from "./JuiceEvent.js";
import { JuiceNode } from "./JuiceNode.js";

export type JuiceElementProps = {
  [K in keyof UIEventMap as `on${Capitalize<K>}`]?: UIEventListener<K>;
} & {
  style?: JuiceElementStyle;
  children?: ComponentChildren;
};

export interface JuiceElementStyle {
  display?: "block" | "flex" | "grid" | "none";
  alignItems?: "stretch" | "flex-start" | "center" | "flex-end";
  alignSelf?: "stretch" | "flex-start" | "center" | "flex-end";
  background?: string;
  borderRadius?: number;
  color?: string;
  flexDirection?: "row" | "column";
  flexGrow?: number;
  flexShrink?: number;
  flexBasis?: number;
  font?: string;
  fontSize?: number;
  gap?: number;
  gapWidth?: number;
  gapHeight?: number;
  height?: string | number;
  justifyContent?:
    | "stretch"
    | "flex-start"
    | "center"
    | "flex-end"
    | "space-between"
    | "space-around";
  justifySelf?: "stretch" | "flex-start" | "center" | "flex-end";
  textAlign?: "left" | "center" | "right";
  margin?: number;
  marginBottom?: number;
  marginLeft?: number;
  marginRight?: number;
  marginTop?: number;
  marginX?: number;
  marginY?: number;
  padding?: number;
  paddingBottom?: number;
  paddingLeft?: number;
  paddingRight?: number;
  paddingTop?: number;
  paddingX?: number;
  paddingY?: number;
  width?: string | number;
}

export class JuiceElement<
  Props extends JuiceElementProps = JuiceElementProps,
> extends JuiceNode {
  public static readonly namespace: string = "http://www.w3.org/1999/xhtml";

  public readonly tagName: string;
  public readonly props: Partial<Props> = {};
  public readonly style: Record<string, unknown>;

  constructor(tagName: string, namespaceURI: string | undefined = undefined) {
    super(JuiceNode.ELEMENT_NODE, namespaceURI);
    this.tagName = tagName;
    this.style = new Proxy({}, this.handleStyleProxy);
  }

  setAttribute(key: string, value: unknown): void {
    (this.props as Record<string, unknown>)[key] = value;
    if (!this.nodeId) return;

    if (typeof value === "string") {
      const match = value.match(/^(\d+(\.\d+)?)(px)$/);

      if (!match) {
        dom.setAttributeString(this.nodeId, key, value);
      } else if (match[3] === "px") {
        dom.setAttributeNumber(this.nodeId, key, parseFloat(match[1]));
      }
    } else if (typeof value === "number") {
      dom.setAttributeNumber(this.nodeId, key, value);
    }
  }

  removeAttribute(key: string): void {
    delete (this.props as Record<string, unknown>)[key];
  }

  findElementByNodeId(id: number): JuiceNode | undefined {
    if (this.nodeId === id) {
      return this;
    }

    for (const child of this.childNodes) {
      if (child.nodeId === id) {
        return child;
      }

      if (child instanceof JuiceElement) {
        const found = child.findElementByNodeId(id);

        if (found) {
          return found;
        }
      }
    }

    return undefined;
  }

  private handleStyleProxy: ProxyHandler<Record<string, unknown>> = {
    set: (style, key, value) => {
      if (typeof key === "symbol") return false;
      style[key] = value;

      if (
        ["background", "borderRadius", "font", "fontSize", "color"].includes(
          key,
        )
      ) {
        this.setAttribute(key, value);
        return true;
      }

      if (key === "marginX") {
        this.style.marginLeft = value;
        this.style.marginRight = value;
      } else if (key === "marginY") {
        this.style.marginTop = value;
        this.style.marginBottom = value;
      } else if (key === "margin") {
        this.style.marginTop = value;
        this.style.marginRight = value;
        this.style.marginBottom = value;
        this.style.marginLeft = value;
      } else if (key === "paddingX") {
        this.style.paddingLeft = value;
        this.style.paddingRight = value;
      } else if (key === "paddingY") {
        this.style.paddingTop = value;
        this.style.paddingBottom = value;
      } else if (key === "padding") {
        this.style.paddingTop = value;
        this.style.paddingRight = value;
        this.style.paddingBottom = value;
        this.style.paddingLeft = value;
      } else if (key === "gap") {
        this.style.gapHeight = value;
        this.style.gapWidth = value;
      } else if (this.nodeId) {
        if (typeof value === "string") {
          const match = value.match(/^(\d+(\.\d+)?)(px|%|em)$/);

          if (!match) {
            const match = value.match(/^#([0-9a-fA-F]{3})$/);

            if (!match) {
              dom.setStyleString(this.nodeId, key, value);
            } else {
              const [r, g, b] = match[1];
              dom.setStyleString(this.nodeId, key, `#${r}${r}${g}${g}${b}${b}`);
            }
          } else if (match[3] === "px") {
            dom.setStyleNumber(this.nodeId, key, parseFloat(match[1]));
          } else if (match[3] === "%") {
            dom.setStylePercent(this.nodeId, key, parseFloat(match[1]));
          } else if (match[3] === "em") {
            dom.setStyleEm(this.nodeId, key, parseFloat(match[1]));
          }
        } else if (typeof value === "number") {
          dom.setStyleNumber(this.nodeId, key, value);
        }
      }

      return true;
    },
  };

  toJSON() {
    // make it easy to dump the DOM for debugging
    return {
      id: this.nodeId,
      tag: this.tagName,
      props: this.props,
      style: this.style,
      children: this.childNodes,
    };
  }
}
