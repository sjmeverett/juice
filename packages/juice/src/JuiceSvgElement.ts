import { JuiceElement, type JuiceElementProps } from "./JuiceElement.js";
import JuiceLayoutElement from "./JuiceLayoutElement.js";
import type { JuiceNode } from "./JuiceNode.js";

interface ExtraSvgProps {
  markup?: string;
}

declare module "preact" {
  namespace JSX {
    interface SVGAttributes extends ExtraSvgProps {}
  }
}

export class JuiceSvgElement extends JuiceLayoutElement<
  JuiceElementProps & preact.SVGAttributes & ExtraSvgProps
> {
  public static readonly namespace = "http://www.w3.org/2000/svg";

  constructor() {
    super("svg");
  }

  private update() {
    dom.setAttributeString(
      this.nodeId,
      "markup",
      toSvgString(this, this.props.markup),
    );
  }

  setAttribute(key: string, value: unknown): void {
    super.setAttribute(key, value);
    this.update();
  }

  appendChild(node: JuiceNode) {
    const result = super.appendChild(node);
    this.update();
    return result;
  }

  removeChild(child: JuiceNode) {
    const result = super.removeChild(child);
    this.update();
    return result;
  }
}

function toSvgString(element: JuiceElement, markup?: string): string {
  let xml = `<${element.tagName}`;

  for (const [key, value] of Object.entries(element.props)) {
    if (typeof value !== "function" && value != null && key !== "markup") {
      xml += ` ${svgAttrName(key)}="${escapeXmlAttr(String(value))}"`;
    }
  }

  if (markup) {
    xml += `>${markup}</${element.tagName}>`;
  } else if (element.childNodes.length === 0) {
    xml += `/>`;
  } else {
    xml += `>`;

    for (const child of element.childNodes) {
      if (child instanceof JuiceElement) {
        xml += toSvgString(child);
      }
    }

    xml += `</${element.tagName}>`;
  }

  return xml;
}

// SVG attributes that must remain camelCase in XML output
const camelCaseAttributes = new Set([
  "viewBox",
  "preserveAspectRatio",
  "baseFrequency",
  "clipPathUnits",
  "filterUnits",
  "gradientTransform",
  "gradientUnits",
  "lengthAdjust",
  "markerHeight",
  "markerUnits",
  "markerWidth",
  "maskContentUnits",
  "maskUnits",
  "numOctaves",
  "pathLength",
  "patternContentUnits",
  "patternTransform",
  "patternUnits",
  "pointsAtX",
  "pointsAtY",
  "pointsAtZ",
  "repeatCount",
  "repeatDur",
  "specularConstant",
  "specularExponent",
  "spreadMethod",
  "startOffset",
  "stdDeviation",
  "stitchTiles",
  "surfaceScale",
  "tableValues",
  "textLength",
  "xChannelSelector",
  "yChannelSelector",
]);

function svgAttrName(jsxName: string): string {
  if (camelCaseAttributes.has(jsxName)) {
    return jsxName;
  } else {
    return jsxName.replace(/([a-z])([A-Z])/g, "$1-$2").toLowerCase();
  }
}

function escapeXmlAttr(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}
