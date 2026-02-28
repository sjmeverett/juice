import { UIElement, type UIElementProps } from "./UIElement.js";
import { acceptPixelsOrPercent } from "./util.js";

interface ExtraSvgProps {
  markup?: string;
}

declare module "preact" {
  namespace JSX {
    interface SVGAttributes extends ExtraSvgProps {}
  }
}

export class UISvgElement extends UIElement<
  UIElementProps & preact.SVGAttributes & ExtraSvgProps
> {
  public static readonly namespace = "http://www.w3.org/2000/svg";

  constructor(tagName: string) {
    super(tagName, UISvgElement.namespace);
  }

  toJSON() {
    if (this.tagName === "svg") {
      return {
        type: "svg",
        id: this.nodeId,
        markup: this.toSvgString(),
        width: acceptPixelsOrPercent(this.props.width),
        height: acceptPixelsOrPercent(this.props.height),
      };
    }

    // Non-root SVG elements (shouldn't be reached since parent consumes them,
    // but fall back to default serialization just in case)
    return super.toJSON();
  }

  toSvgString(): string {
    const tag = this.tagName;
    let xml = `<${tag}`;

    for (const [key, value] of Object.entries(this.props)) {
      if (typeof value !== "function" && value != null && key !== "markup") {
        xml += ` ${svgAttrName(key)}="${escapeXmlAttr(String(value))}"`;
      }
    }

    if (this.props.markup) {
      xml += `>${this.props.markup}</${tag}>`;
    } else if (this.childNodes.length === 0) {
      xml += `/>`;
    } else {
      xml += `>`;

      for (const child of this.childNodes) {
        if (child instanceof UISvgElement) {
          xml += child.toSvgString();
        }
      }

      xml += `</${tag}>`;
    }

    return xml;
  }
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
