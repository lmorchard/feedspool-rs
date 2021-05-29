import config from "./config.js";

export const queryFetchFeeds = gql`
  query fetchFeeds($since: DateTimeUtc!, $takeFeeds: Int!) {
    feeds(pagination: { take: $takeFeeds }, since: $since) {
      id
      title
      link
      published
      lastEntryPublished
      entries(since: $since) {
        id
        title
        published
        link
        content
      }
    }
  }
`;

export async function gqlFetch(query, variables = null) {
  const resp = await fetch(config.GRAPHQL_BASE_URL, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query, variables }),
  });
  if (resp.status !== 200) {
    throw new Error(`${resp.status} ${resp.statusText} ${await resp.text()}`);
  }
  return await resp.json();
}

export function gql(strings, ...values) {
  const query = strings
    .reduce(
      (result, string, idx) =>
        result + string + (values[idx] ? values[idx] : ""),
      ""
    )
    .trim();
  return async (variables = null) => gqlFetch(query, variables);
}
