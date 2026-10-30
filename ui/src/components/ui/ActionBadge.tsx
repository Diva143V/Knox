import React from "react";

interface ActionBadgeProps {
  type: string;
  className?: string;
}

export const ActionBadge: React.FC<ActionBadgeProps> = ({ type, className = "" }) => {
  const norm = (type || "").toLowerCase();

  let bg = "bg-indigo-500/15 text-indigo-400 border-indigo-500/30";
  if (norm.includes("write")) {
    bg = "bg-emerald-500/15 text-emerald-400 border-emerald-500/30";
  } else if (norm.includes("plan")) {
    bg = "bg-purple-500/15 text-purple-400 border-purple-500/30";
  } else if (norm.includes("decide")) {
    bg = "bg-amber-500/15 text-amber-400 border-amber-500/30";
  } else if (norm.includes("debug")) {
    bg = "bg-rose-500/15 text-rose-400 border-rose-500/30";
  }

  return (
    <span
      className={`inline-flex items-center px-2 py-0.5 rounded text-[11px] font-medium tracking-wide uppercase border font-mono ${bg} ${className}`}
    >
      {type}
    </span>
  );
};

