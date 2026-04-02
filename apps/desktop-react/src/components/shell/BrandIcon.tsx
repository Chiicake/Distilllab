type BrandIconProps = {
  className?: string;
};

export default function BrandIcon({ className = '' }: BrandIconProps) {
  return (
    <div
      aria-hidden="true"
      className={`flex h-8 w-8 items-center justify-center rounded-sm bg-gradient-to-br from-primary/90 to-primary-container ${className}`.trim()}
    >
      <div className="relative h-[20px] w-[20px]">
        <span className="absolute left-0 top-0 h-[5px] w-[20px] rounded-full bg-on-primary-container/42" />
        <span className="absolute left-[3px] top-[8px] h-[5px] w-[14px] rounded-full bg-on-primary-container/68" />
        <span className="absolute left-[7px] top-[15px] h-[5px] w-[6px] rounded-full bg-on-primary-container" />
      </div>
    </div>
  );
}
