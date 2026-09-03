import { useEffect, type ReactNode } from "react"
import { createPortal } from "react-dom"
import { X } from "lucide-react"

export function Dialog({ open, onClose, title, children }: { open: boolean; onClose: () => void; title: string; children: ReactNode }) {
  useEffect(() => {
    if (!open) return
    const close = (event: KeyboardEvent) => { if (event.key === "Escape") onClose() }
    window.addEventListener("keydown", close)
    return () => window.removeEventListener("keydown", close)
  }, [open, onClose])
  useEffect(() => {
    if (!open) return
    const previousOverflow = document.body.style.overflow
    document.body.style.overflow = "hidden"
    return () => { document.body.style.overflow = previousOverflow }
  }, [open])
  if (!open) return null
  return createPortal(<div className="fixed inset-0 z-[100] grid place-items-center bg-slate-950/80 p-4 backdrop-blur-sm" onMouseDown={onClose}>
    <section role="dialog" aria-modal="true" aria-labelledby="dialog-title" className="w-full max-w-2xl rounded-2xl border border-slate-700 bg-slate-900 p-6 shadow-2xl" onMouseDown={(event) => event.stopPropagation()}>
      <div className="flex items-start justify-between gap-4"><h2 id="dialog-title" className="text-xl font-semibold text-white">{title}</h2><button aria-label="Zamknij" className="rounded-lg p-2 text-slate-400 hover:bg-slate-800 hover:text-white" onClick={onClose}><X className="size-5" /></button></div>
      <div className="mt-4">{children}</div>
    </section>
  </div>, document.body)
}
