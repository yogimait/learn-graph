export interface Domain {
  id: number;
  name: string;
  color: string;
  description: string;
  sort: number;
}

export interface Subdomain {
  id: number;
  domain_id: number;
  name: string;
  sort: number;
}

export interface DomainWithSubs extends Domain {
  subdomains: Subdomain[];
}

export interface Classification {
  domain_id: number;
  domain_name: string;
  subdomain_id: number | null;
  subdomain_name: string | null;
  probability: number;
}

export interface Event {
  id: number;
  raw_text: string;
  canonical_topic: string;
  event_date: string;
  status: string;
  confidence: number;
  source: string;
  created_at: string;
}

export interface EventWithClassifications extends Event {
  classifications: Classification[];
}

export interface CaptureResult {
  event_id: number;
  status: string;
}

export interface DayCount {
  date: string;
  count: number;
}

/** get_settings returns a flat [key, value] pair list. */
export type Settings = Record<string, string>;