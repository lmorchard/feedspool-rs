import { createElement } from "./lib/dom.js";
import { queryFetchFeeds } from "./lib/gql.js";
import "./lib/components.js";

async function main() {
  const now = Date.now();
  const since = new Date(now - 1000 * 60 * 60 * 24);
  const result = await queryFetchFeeds({
    takeFeeds: 250,
    since: since.toISOString()// .replace(/Z$/, "+00:00"),
  });
  document.body.append(
    createElement("feed-reader", {
      id: "app",
      props: { since, ...result.data },
    })
  );
}

document.addEventListener("DOMContentLoaded", () =>
  main()
    .then(() => console.log("READY."))
    .catch((err) => console.error(err))
);
