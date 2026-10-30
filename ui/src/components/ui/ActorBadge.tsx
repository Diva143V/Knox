import React from "react";

interface ActorBadgeProps {
  actor: string;
  className?: string;
}

export const ActorBadge: React.FC<ActorBadgeProps> = ({ actor, className = "" }) => {
  const norm = (actor || "").toLowerCase();

  let style = "bg-slate-800 text-slate-300 border-slate-700";
  if (norm === "human") {
    style = "bg-pink-500/15 text-pink-400 border-pink-500/30";
  } else if (norm === "gemini") {
    style = "bg-blue-500/15 text-blue-400 border-blue-500/30";
  } else if (norm === "cursor") {
    style = "bg-purple-500/15 text-purple-400 border-purple-500/30";
  } else if (norm === "antigravity") {
    style = "bg-indigo-500/15 text-indigo-400 border-indigo-500/30";
  } else if (norm === "copilot") {
    style = "bg-sky-500/15 text-sky-400 border-sky-500/30";
  }

  return (
    <span
      className={`inline-flex items-center gap-1 px-2 py-0.5 rounded text-[11px] font-medium border font-mono ${style} ${className}`}
    >
      <span className="w-1.5 h-1.5 rounded-full bg-current opacity-70" />
      {actor}
    </span>
  );
};

