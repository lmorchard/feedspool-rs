import config from "./config.js";

export const queryFetchFeeds = gql`
  query fetchFeeds($since: DateTimeUtc!, $takeFeeds: Int!) {
    feeds(pagination: { take: $takeFeeds }, since: $since) {
      title
      entries(since: $since) {
        title
        published
        link
        content
      }
    }
  }
`;

export function gql(strings, ...values) {
  const query = strings
    .reduce(
      (result, string, idx) =>
        result + string + (values[idx] ? values[idx] : ""),
      ""
    )
    .trim();

  return async (variables = null) => {
    const resp = await fetch(config.GRAPHQL_BASE_URL, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ query, variables }),
    });
    const result = await resp.json();
    return result;
  };
}
