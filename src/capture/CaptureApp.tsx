import { useEffect, useRef, useState } from "react";
import { api, onCaptureOpen } from "../lib/api";

export default function CaptureApp() {
  const [text, setText] = useState("");
  const [status, setStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [error, setError] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const timeRef = useRef<number | null>(null);

  const reset = () => {
    setText("");
    setStatus("idle");
    setError("");
    inputRef.current?.focus();
  };

  useEffect(() => {
    const unlisten = onCaptureOpen(() => {
      reset();
      inputRef.current?.focus();
      inputRef.current?.select();
    });
    inputRef.current?.focus();
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  useEffect(() => {
    if (timeRef.current) window.clearTimeout(timeRef.current);
    if (status === "saved") {
      timeRef.current = window.setTimeout(() => api.hideCapture(), 900);
    }
    return () => {
      if (timeRef.current) window.clearTimeout(timeRef.current);
    };
  }, [status]);

  const submit = async () => {
    const value = text.trim();
    if (!value || status === "saving") return;
    setStatus("saving");
    try {
      await api.capture(value);
      setStatus("saved");
    } catch (e) {
      setStatus("error");
      setError(String(e));
    }
  };

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      submit();
    } else if (e.key === "Escape") {
      api.hideCapture();
      reset();
    }
  };

  return (
    <div className={`capture ${status === "saved" ? "capture-saved" : ""}`}>
      <input
        ref={inputRef}
        value={text}
        onChange={(e) => {
          setText(e.target.value);
          if (status !== "idle") setStatus("idle");
        }}
        onKeyDown={onKey}
        placeholder="What did you learn or practice?"
        autoFocus
        spellCheck={false}
      />
      <div className="capture-status">
        {status === "saving" && <span className="muted">classifying…</span>}
        {status === "saved" && <span className="ok">✓ saved</span>}
        {status === "error" && <span className="err">{error}</span>}
      </div>
    </div>
  );
}