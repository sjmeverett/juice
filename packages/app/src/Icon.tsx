import { type IconifyIcon, iconToSVG } from "@iconify/utils";

type IconInput = IconifyIcon | { default: IconifyIcon; __esModule?: boolean };

export interface IconProps {
  icon: IconInput;
  size?: number;
}

export default function Icon({ icon, size = 24 }: IconProps) {
  const _icon = 'body' in icon ? icon : icon.default;
  const svg = iconToSVG(_icon, { height: size });

  return (
    <svg {...svg.attributes} markup={svg.body} />
  );
}
