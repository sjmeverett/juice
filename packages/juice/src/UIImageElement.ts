import { UIElement, type UIElementProps } from "./UIElement.js";
import { acceptPixelsOrPercent } from "./util.js";

interface ImageProps extends UIElementProps {
  src?: string;
  width?: number | string;
  height?: number | string;
}

export class UIImageElement extends UIElement<ImageProps> {
  constructor() {
    super("img");
  }

  toJSON() {
    return {
      type: "image",
      id: this.nodeId,
      src: this.props.src,
      width: acceptPixelsOrPercent(this.props.width),
      height: acceptPixelsOrPercent(this.props.height),
    };
  }
}
