export type QueryParameter = { name: string; value: string };

export type ComposeResult = {
  status: "ready" | "failed";
  value: string | null;
  message: string | null;
};

export function parseWebUrl(value: string): URL | null {
  if (!/^(?:http|https):\/\/[^/?#]+(?:[/?#]|$)/.test(value)) return null;
  try {
    return new URL(value);
  } catch {
    return null;
  }
}

export function composeUrl(baseUrl: string, parameters: QueryParameter[]): ComposeResult {
  if (parseWebUrl(baseUrl) === null) {
    return {
      status: "failed",
      value: null,
      message: "Enter a valid URL beginning with http:// or https://.",
    };
  }

  for (const parameter of parameters) {
    if (parameter.name.length === 0) {
      if (parameter.value.length === 0) continue;
      return {
        status: "failed",
        value: null,
        message: "Enter a name for each custom parameter that has a value.",
      };
    }
  }

  return { status: "ready", value: baseUrl, message: null };
}

function querySegmentName(segment: string): string | null {
  const first = new URLSearchParams(segment).keys().next();
  return first.done ? null : first.value;
}

export function updateQueryParameter(baseUrl: string, name: string, value: string | null): string {
  const fragmentIndex = baseUrl.indexOf("#");
  const withoutFragment = fragmentIndex === -1 ? baseUrl : baseUrl.slice(0, fragmentIndex);
  const fragment = fragmentIndex === -1 ? "" : baseUrl.slice(fragmentIndex);
  const queryIndex = withoutFragment.indexOf("?");
  const prefix = queryIndex === -1 ? withoutFragment : withoutFragment.slice(0, queryIndex);
  const query = queryIndex === -1 ? "" : withoutFragment.slice(queryIndex + 1);
  const encoded = value === null ? null : new URLSearchParams([[name, value]]).toString();
  const segments: string[] = [];
  let replaced = false;

  for (const segment of query.length === 0 ? [] : query.split("&")) {
    if (querySegmentName(segment) !== name) {
      segments.push(segment);
    } else if (!replaced && encoded !== null) {
      segments.push(encoded);
      replaced = true;
    }
  }
  if (!replaced && encoded !== null) segments.push(encoded);

  return `${prefix}${segments.length === 0 ? "" : `?${segments.join("&")}`}${fragment}`;
}

export function updateQueryParameterIfChanged(
  baseUrl: string,
  name: string,
  value: string | null,
): string {
  const parsed = parseWebUrl(baseUrl);
  if (parsed === null) return baseUrl;
  if (value === null ? !parsed.searchParams.has(name) : parsed.searchParams.get(name) === value) {
    return baseUrl;
  }
  return updateQueryParameter(baseUrl, name, value);
}
