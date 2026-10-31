import { Component, ErrorInfo, ReactNode } from "react";
import { AlertTriangle, RefreshCw, RotateCcw, Copy, Check } from "lucide-react";

interface Props {
  children: ReactNode;
  fallbackTitle?: string;
}

interface State {
  hasError: boolean;
  error: Error | null;
  errorInfo: ErrorInfo | null;
  copied: boolean;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
    errorInfo: null,
    copied: false,
  };

  public static getDerivedStateFromError(error: Error): Partial<State> {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Unhandled error caught by Knox ErrorBoundary:", error, errorInfo);
    this.setState({ errorInfo });
  }

  private handleReload = () => {
    window.location.reload();
  };

  private handleResetAndReload = () => {
    try {
      localStorage.clear();
    } catch {
      // ignore
    }
    window.location.href = window.location.pathname;
  };

  private handleCopy = () => {
    const diagnostic = `Knox Error Diagnostic:
Timestamp: ${new Date().toISOString()}
Error: ${this.state.error?.message || "Unknown error"}
Stack:
${this.state.error?.stack || "No stack trace"}
Component Stack:
${this.state.errorInfo?.componentStack || "No component stack"}
User Agent: ${navigator.userAgent}
Location: ${window.location.href}`;

    navigator.clipboard.writeText(diagnostic).then(() => {
      this.setState({ copied: true });
      setTimeout(() => this.setState({ copied: false }), 2000);
    });
  };

  public render() {
    if (this.state.hasError) {
      return (
        <div className="h-screen w-screen flex items-center justify-center bg-[#0d1117] text-[#f8fafc] p-6 select-none">
          <div className="max-w-xl w-full bg-[#161b22] border border-[#30363d] rounded-2xl p-6 shadow-2xl space-y-5 animate-in fade-in duration-200">
            <div className="flex items-center gap-3">
              <div className="p-2.5 rounded-xl bg-rose-500/15 text-rose-400 border border-rose-500/30">
                <AlertTriangle className="w-6 h-6" />
              </div>
              <div>
                <h1 className="text-base font-semibold text-white tracking-tight">
                  {this.props.fallbackTitle || "Knox Encountered a Render Error"}
                </h1>
                <p className="text-xs text-slate-400">
                  A component error occurred during rendering. The application has been halted to prevent corrupt state.
                </p>
              </div>
            </div>

            <div className="p-3.5 rounded-xl bg-[#0d1117] border border-[#30363d] font-mono text-xs text-rose-300 overflow-x-auto max-h-48 whitespace-pre-wrap break-all">
              {this.state.error?.message || "Unknown error"}
              {this.state.error?.stack && (
                <span className="block mt-2 text-[11px] text-slate-500 font-sans">
                  {this.state.error.stack}
                </span>
              )}
            </div>

            <div className="flex flex-wrap items-center justify-between gap-3 pt-2">
              <div className="flex items-center gap-2">
                <button
                  onClick={this.handleReload}
                  className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-semibold bg-indigo-600 hover:bg-indigo-500 text-white shadow-md transition cursor-pointer"
                >
                  <RefreshCw className="w-3.5 h-3.5" />
                  Reload App
                </button>
                <button
                  onClick={this.handleResetAndReload}
                  className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-semibold bg-[#21262d] hover:bg-[#30363d] text-slate-200 border border-[#30363d] transition cursor-pointer"
                  title="Clear localStorage (resets onboarding) and reloads"
                >
                  <RotateCcw className="w-3.5 h-3.5" />
                  Reset & Restart
                </button>
              </div>

              <button
                onClick={this.handleCopy}
                className="flex items-center gap-1.5 px-3 py-2 rounded-xl text-xs font-medium text-slate-400 hover:text-white transition cursor-pointer"
              >
                {this.state.copied ? (
                  <>
                    <Check className="w-3.5 h-3.5 text-emerald-400" />
                    <span className="text-emerald-400">Copied</span>
                  </>
                ) : (
                  <>
                    <Copy className="w-3.5 h-3.5" />
                    <span>Copy Error</span>
                  </>
                )}
              </button>
            </div>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
