import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  CaptureResult,
  DayCount,
  DomainWithSubs,
  EventWithClassifications,
} from "./types";

export const api = {
  capture: (text: string, date?: string) =>
    invoke<CaptureResult>("capture_event", { req: { text, date } }),
  review: (eventId: number, domainIds: number[], subdomainIds: (number | null)[]) =>
    invoke<void>("review_event", {
      update: { event_id: eventId, domain_ids: domainIds, subdomain_ids: subdomainIds },
    }),
  listDomains: () => invoke<DomainWithSubs[]>("list_domains"),
  addDomain: (name: string, color: string, description: string) =>
    invoke<number>("add_domain", { domain: { name, color, description } }),
  updateDomain: (id: number, name: string, description: string) =>
    invoke<void>("update_domain", { id, name, description }),
  deleteDomain: (id: number) => invoke<void>("delete_domain", { id }),
  addSubdomain: (domainId: number, name: string) =>
    invoke<number>("add_subdomain", { sub: { domain_id: domainId, name } }),
  deleteSubdomain: (id: number) => invoke<void>("delete_subdomain", { id }),
  listEvents: (status: string | null, limit: number) =>
    invoke<EventWithClassifications[]>("list_events", { status, limit }),
  sendToReview: (id: number) => invoke<void>("send_to_review", { id }),
  dailyCounts: (domainId: number | null, from: string, to: string) =>
    invoke<DayCount[]>("daily_counts", { domainId, from, to }),
  getSettings: () => invoke<[string, string][]>("get_settings"),
  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),
  daemonHealth: () => invoke<{ status?: string; device?: string }>("daemon_health"),
  hideCapture: () => invoke<void>("hide_capture_window"),
};

export const onCaptureOpen = (cb: () => void) => listen("capture:open", cb);
export const onClassified = (cb: (eventId: number) => void) =>
  listen<number>("event:classified", (e) => cb(e.payload));