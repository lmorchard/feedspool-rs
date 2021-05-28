import { html, createElement, BaseElement } from "./dom.js";

customElements.define(
  "feed-reader",
  Object.assign(
    class extends BaseElement {
      propsChanged({ since, feeds }) {
        this.$('.since').textContent = since;
        this.replaceChildren(
          ".feeds",
          feeds.map((feed) =>
            createElement("feed-reader-feed", { props: feed })
          )
        );
      }
    },
    {
      template: html`
        <template>
          <style>
            :host > header {
              margin: 1em;
            }
            .feeds {
              margin: 0.5em;
            }
          </style>
          <header>Since: <span class="since"></span></header>
          <section class="feeds"></section>
        </template>
      `,
    }
  )
);

customElements.define(
  "feed-reader-feed",
  Object.assign(
    class extends BaseElement {
      propsChanged({ title, entries }) {
        this.$(".title").textContent = title;
        this.replaceChildren(
          ".entries",
          entries.map((entry) =>
            createElement("feed-reader-entry", { props: entry })
          )
        );
      }
    },
    {
      template: html`
        <template>
          <style>
            .feed {
              padding: 0.5em;
              margin: 0;
            }
            .feed > .title {
              display: block;
              width: calc(100% - 1em);
              padding: 0.5em;
              margin: 0.25em;
              border: 1px solid #000;
              box-shadow: 4px 4px 3px rgba(0, 0, 0, 0.4);
            }
            .feed > .entries {
              width: 100%;
              margin: 1em 0.25em;
              display: flex;
              flex-direction: row;
              flex-wrap: wrap;
              justify-content: flex-start;
              align-items: stretch;
              align-content: center;
            }
            .feed > .entries > feed-reader-entry {
              width: 25%;

            }
          </style>
          <section class="feed">
            <span class="title">Title</span>
            <div class="entries"></div>
          </section>
        </template>
      `,
    }
  )
);

customElements.define(
  "feed-reader-entry",
  Object.assign(
    class extends BaseElement {
      propsChanged({ title, link, content, published }) {
        const elTitle = this.$(".title a");
        elTitle.textContent = title;
        elTitle.setAttribute("href", link);
        
        const elPublished = this.$(".published a");
        elPublished.textContent = published;
        elPublished.setAttribute("href", link);

        if (!content) {
          this.clearChildren(".content");
        } else {
          const iframe = createElement("iframe", {
            frameBorder: "0",
            src: "data:text/html;charset=utf-8," + encodeURI(content),
          });
          this.replaceChildren(".content", [iframe]);
          iframe.setAttribute("style", "height:100%;width:100%;");
        }
      }
    },
    {
      template: html`
        <template>
          <style>
            .entry {
              padding: 0.5em;
              margin: 0.5em;
              border: 1px solid #000;
              box-shadow: 4px 4px 3px rgba(0, 0, 0, 0.4);
            }
          </style>
          <div class="entry">
            <div class="title"><a href="">Title</a></div>
            <span class="published" datetime=""><a href=""></a></span>
            <div class="content"></div>
          </div>
        </template>
      `,
    }
  )
);
