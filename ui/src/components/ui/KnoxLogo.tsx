import React, { useId } from "react";

export interface KnoxLogoProps {
  size?: number;
  variant?: "card" | "mark";
  className?: string;
  glow?: boolean;
}

export const KnoxLogo: React.FC<KnoxLogoProps> = ({
  size = 24,
  variant = "mark",
  className = "",
  glow = true,
}) => {
  const rawId = useId();
  const id = rawId.replace(/[^a-zA-Z0-9-_]/g, "");

  const bgGradId = `knox-bg-${id}`;
  const glowFilterId = `knox-glow-${id}`;
  const maskAId = `knox-cutA-${id}`;
  const maskBId = `knox-cutB-${id}`;

  if (variant === "card") {
    return (
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 500 500"
        width={size}
        height={size}
        role="img"
        aria-label="Knox"
        className={`shrink-0 select-none ${className}`}
      >
        <defs>
          <radialGradient id={bgGradId} cx="50%" cy="46%" r="65%">
            <stop offset="0" stopColor="#FF073A" stopOpacity=".24" />
            <stop offset="1" stopColor="#FF073A" stopOpacity="0" />
          </radialGradient>
          {glow && (
            <filter id={glowFilterId} x="-30%" y="-30%" width="160%" height="160%">
              <feGaussianBlur stdDeviation="9" />
            </filter>
          )}
          <mask id={maskAId} maskUnits="userSpaceOnUse" x="0" y="0" width="500" height="500">
            <rect width="500" height="500" fill="#fff" />
            <path
              d="M 75 360 L 97.5 360.0 Q 120 360 130.8 340.2 L 230.4 157.6 Q 240 140 260.0 140.0 L 260.0 140.0 Q 280 140 289.6 157.6 L 390.4 342.4 Q 400 360 420.0 360.0 L 440 360"
              fill="none"
              stroke="#000"
              strokeWidth="38"
            />
            <rect width="260" height="500" fill="#fff" />
          </mask>
          <mask id={maskBId} maskUnits="userSpaceOnUse" x="0" y="0" width="500" height="500">
            <rect width="500" height="500" fill="#fff" />
            <path
              d="M 75 140 L 97.5 140.0 Q 120 140 130.8 159.8 L 230.4 342.4 Q 240 360 260.0 360.0 L 260.0 360.0 Q 280 360 289.6 342.4 L 390.4 157.6 Q 400 140 420.0 140.0 L 440 140"
              fill="none"
              stroke="#000"
              strokeWidth="38"
            />
            <rect x="260" width="240" height="500" fill="#fff" />
          </mask>
        </defs>

        {/* Squircle Card Container */}
        <rect width="500" height="500" rx="110" fill="#07070D" />
        <rect width="500" height="500" rx="110" fill={`url(#${bgGradId})`} />
        <rect
          x="3"
          y="3"
          width="494"
          height="494"
          rx="107"
          fill="none"
          stroke="#FF073A"
          strokeOpacity=".35"
          strokeWidth="2"
        />

        {/* Interlocking Geometry */}
        <g transform="translate(-8 0)" fill="none" strokeLinejoin="round">
          <g mask={`url(#${maskBId})`}>
            {glow && (
              <path
                d="M 75 360 L 97.5 360.0 Q 120 360 130.8 340.2 L 230.4 157.6 Q 240 140 260.0 140.0 L 260.0 140.0 Q 280 140 289.6 157.6 L 390.4 342.4 Q 400 360 420.0 360.0 L 440 360"
                stroke="#FF073A"
                strokeWidth="18"
                filter={`url(#${glowFilterId})`}
              />
            )}
            <path
              d="M 75 360 L 97.5 360.0 Q 120 360 130.8 340.2 L 230.4 157.6 Q 240 140 260.0 140.0 L 260.0 140.0 Q 280 140 289.6 157.6 L 390.4 342.4 Q 400 360 420.0 360.0 L 440 360"
              stroke="#FF073A"
              strokeWidth="8"
            />
            <path
              d="M 75 360 L 97.5 360.0 Q 120 360 130.8 340.2 L 230.4 157.6 Q 240 140 260.0 140.0 L 260.0 140.0 Q 280 140 289.6 157.6 L 390.4 342.4 Q 400 360 420.0 360.0 L 440 360"
              stroke="#FFC8D4"
              strokeWidth="3"
            />
          </g>

          <g mask={`url(#${maskAId})`}>
            {glow && (
              <path
                d="M 75 140 L 97.5 140.0 Q 120 140 130.8 159.8 L 230.4 342.4 Q 240 360 260.0 360.0 L 260.0 360.0 Q 280 360 289.6 342.4 L 390.4 157.6 Q 400 140 420.0 140.0 L 440 140"
                stroke="#FF073A"
                strokeWidth="18"
                filter={`url(#${glowFilterId})`}
              />
            )}
            <path
              d="M 75 140 L 97.5 140.0 Q 120 140 130.8 159.8 L 230.4 342.4 Q 240 360 260.0 360.0 L 260.0 360.0 Q 280 360 289.6 342.4 L 390.4 157.6 Q 400 140 420.0 140.0 L 440 140"
              stroke="#FF073A"
              strokeWidth="8"
            />
            <path
              d="M 75 140 L 97.5 140.0 Q 120 140 130.8 159.8 L 230.4 342.4 Q 240 360 260.0 360.0 L 260.0 360.0 Q 280 360 289.6 342.4 L 390.4 157.6 Q 400 140 420.0 140.0 L 440 140"
              stroke="#FFC8D4"
              strokeWidth="3"
            />
          </g>

          {/* Node Circles */}
          <g>
            {glow && (
              <>
                <circle cx="75" cy="140" r="17" fill="none" stroke="#FF073A" strokeWidth="8" filter={`url(#${glowFilterId})`} />
                <circle cx="440" cy="140" r="17" fill="none" stroke="#FF073A" strokeWidth="8" filter={`url(#${glowFilterId})`} />
                <circle cx="75" cy="360" r="17" fill="none" stroke="#FF073A" strokeWidth="8" filter={`url(#${glowFilterId})`} />
                <circle cx="440" cy="360" r="17" fill="none" stroke="#FF073A" strokeWidth="8" filter={`url(#${glowFilterId})`} />
              </>
            )}
            <circle cx="75" cy="140" r="17" fill="#07070D" stroke="#FF073A" strokeWidth="6" />
            <circle cx="75" cy="140" r="5" fill="#FFC8D4" />
            <circle cx="440" cy="140" r="17" fill="#07070D" stroke="#FF073A" strokeWidth="6" />
            <circle cx="440" cy="140" r="5" fill="#FFC8D4" />
            <circle cx="75" cy="360" r="17" fill="#07070D" stroke="#FF073A" strokeWidth="6" />
            <circle cx="75" cy="360" r="5" fill="#FFC8D4" />
            <circle cx="440" cy="360" r="17" fill="#07070D" stroke="#FF073A" strokeWidth="6" />
            <circle cx="440" cy="360" r="5" fill="#FFC8D4" />
          </g>
        </g>
      </svg>
    );
  }

  // Variant = "mark" (Transparent compact icon)
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="45 105 425 290"
      width={size}
      height={Math.round((size * 290) / 425)}
      role="img"
      aria-label="Knox"
      className={`shrink-0 select-none ${className}`}
    >
      <defs>
        {glow && (
          <filter id={glowFilterId} x="-30%" y="-30%" width="160%" height="160%">
            <feGaussianBlur stdDeviation="9" />
          </filter>
        )}
        <mask id={maskAId} maskUnits="userSpaceOnUse" x="0" y="0" width="500" height="500">
          <rect width="500" height="500" fill="#fff" />
          <path
            d="M 75 360 L 97.5 360.0 Q 120 360 130.8 340.2 L 230.4 157.6 Q 240 140 260.0 140.0 L 260.0 140.0 Q 280 140 289.6 157.6 L 390.4 342.4 Q 400 360 420.0 360.0 L 440 360"
            fill="none"
            stroke="#000"
            strokeWidth="38"
          />
          <rect width="260" height="500" fill="#fff" />
        </mask>
        <mask id={maskBId} maskUnits="userSpaceOnUse" x="0" y="0" width="500" height="500">
          <rect width="500" height="500" fill="#fff" />
          <path
            d="M 75 140 L 97.5 140.0 Q 120 140 130.8 159.8 L 230.4 342.4 Q 240 360 260.0 360.0 L 260.0 360.0 Q 280 360 289.6 342.4 L 390.4 157.6 Q 400 140 420.0 140.0 L 440 140"
            fill="none"
            stroke="#000"
            strokeWidth="38"
          />
          <rect x="260" width="240" height="500" fill="#fff" />
        </mask>
      </defs>

      <g fill="none" strokeLinejoin="round">
        <g mask={`url(#${maskBId})`}>
          {glow && (
            <path
              d="M 75 360 L 97.5 360.0 Q 120 360 130.8 340.2 L 230.4 157.6 Q 240 140 260.0 140.0 L 260.0 140.0 Q 280 140 289.6 157.6 L 390.4 342.4 Q 400 360 420.0 360.0 L 440 360"
              stroke="#FF073A"
              strokeWidth="18"
              filter={`url(#${glowFilterId})`}
            />
          )}
          <path
            d="M 75 360 L 97.5 360.0 Q 120 360 130.8 340.2 L 230.4 157.6 Q 240 140 260.0 140.0 L 260.0 140.0 Q 280 140 289.6 157.6 L 390.4 342.4 Q 400 360 420.0 360.0 L 440 360"
            stroke="#FF073A"
            strokeWidth="8"
          />
          <path
            d="M 75 360 L 97.5 360.0 Q 120 360 130.8 340.2 L 230.4 157.6 Q 240 140 260.0 140.0 L 260.0 140.0 Q 280 140 289.6 157.6 L 390.4 342.4 Q 400 360 420.0 360.0 L 440 360"
            stroke="#FFC8D4"
            strokeWidth="3"
          />
        </g>

        <g mask={`url(#${maskAId})`}>
          {glow && (
            <path
              d="M 75 140 L 97.5 140.0 Q 120 140 130.8 159.8 L 230.4 342.4 Q 240 360 260.0 360.0 L 260.0 360.0 Q 280 360 289.6 342.4 L 390.4 157.6 Q 400 140 420.0 140.0 L 440 140"
              stroke="#FF073A"
              strokeWidth="18"
              filter={`url(#${glowFilterId})`}
            />
          )}
          <path
            d="M 75 140 L 97.5 140.0 Q 120 140 130.8 159.8 L 230.4 342.4 Q 240 360 260.0 360.0 L 260.0 360.0 Q 280 360 289.6 342.4 L 390.4 157.6 Q 400 140 420.0 140.0 L 440 140"
            stroke="#FF073A"
            strokeWidth="8"
          />
          <path
            d="M 75 140 L 97.5 140.0 Q 120 140 130.8 159.8 L 230.4 342.4 Q 240 360 260.0 360.0 L 260.0 360.0 Q 280 360 289.6 342.4 L 390.4 157.6 Q 400 140 420.0 140.0 L 440 140"
            stroke="#FFC8D4"
            strokeWidth="3"
          />
        </g>

        {/* Node Circles */}
        <g>
          {glow && (
            <>
              <circle cx="75" cy="140" r="17" fill="none" stroke="#FF073A" strokeWidth="8" filter={`url(#${glowFilterId})`} />
              <circle cx="440" cy="140" r="17" fill="none" stroke="#FF073A" strokeWidth="8" filter={`url(#${glowFilterId})`} />
              <circle cx="75" cy="360" r="17" fill="none" stroke="#FF073A" strokeWidth="8" filter={`url(#${glowFilterId})`} />
              <circle cx="440" cy="360" r="17" fill="none" stroke="#FF073A" strokeWidth="8" filter={`url(#${glowFilterId})`} />
            </>
          )}
          <circle cx="75" cy="140" r="17" fill="#07070D" stroke="#FF073A" strokeWidth="6" />
          <circle cx="75" cy="140" r="5" fill="#FFC8D4" />
          <circle cx="440" cy="140" r="17" fill="#07070D" stroke="#FF073A" strokeWidth="6" />
          <circle cx="440" cy="140" r="5" fill="#FFC8D4" />
          <circle cx="75" cy="360" r="17" fill="#07070D" stroke="#FF073A" strokeWidth="6" />
          <circle cx="75" cy="360" r="5" fill="#FFC8D4" />
          <circle cx="440" cy="360" r="17" fill="#07070D" stroke="#FF073A" strokeWidth="6" />
          <circle cx="440" cy="360" r="5" fill="#FFC8D4" />
        </g>
      </g>
    </svg>
  );
};

