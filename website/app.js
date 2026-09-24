// hushtype site: platform-aware download dialogs, copy buttons, and the demo.

const root = document.documentElement;

/* Platform ----------------------------------------------------------------- */

function detectPlatform() {
  const ua = navigator.userAgent;
  const platform =
    navigator.userAgentData?.platform || navigator.platform || "";
  // iPadOS reports itself as a Mac; touch support gives it away.
  if (
    /iPhone|iPad|iPod/.test(ua) ||
    (/Macintosh/.test(ua) && navigator.maxTouchPoints > 1)
  ) {
    return "ios";
  }
  if (/Android/i.test(ua)) return "android";
  if (/Win/i.test(platform) || /Windows/.test(ua)) return "windows";
  if (/Mac/i.test(platform) || /Macintosh/.test(ua)) return "mac";
  if (/Linux|X11|CrOS/i.test(platform + ua)) return "linux";
  return "other";
}

const platform = detectPlatform();
root.dataset.platform = platform;

const NOTES = {
  windows:
    "You're on Windows. hushtype runs on Mac today, and <strong>Windows support is coming soon</strong>.",
  ios: "You're on an iPhone or iPad. <strong>hushtype for iPhone is on the way.</strong> Install it on your Mac today.",
  android:
    "hushtype runs on Mac today. Windows support is coming soon, and an iPhone version is on the way.",
  linux:
    "hushtype runs on Mac today. Windows support is coming soon, and an iPhone version is on the way.",
  other:
    "hushtype runs on Mac today. Windows support is coming soon, and an iPhone version is on the way.",
};

if (platform !== "mac") {
  const note = document.querySelector("[data-platform-note]");
  if (note) note.innerHTML = NOTES[platform];
  const label = document.querySelector("[data-download-label]");
  if (label)
    label.textContent =
      platform === "windows" ? "Download for Windows" : "Download";
}

// Chrome and Edge can tell Apple silicon from Intel; Safari can't, so this is best effort.
if (platform === "mac" && navigator.userAgentData?.getHighEntropyValues) {
  navigator.userAgentData
    .getHighEntropyValues(["architecture"])
    .then(({ architecture }) => {
      if (architecture === "x86") {
        document
          .querySelector("[data-intel-warning]")
          ?.removeAttribute("hidden");
      }
    })
    .catch(() => {});
}

/* Dialogs ------------------------------------------------------------------ */

const DIALOG_FOR_PLATFORM = {
  mac: "dialog-mac",
  windows: "dialog-windows",
  ios: "dialog-ios",
  android: "dialog-other",
  linux: "dialog-other",
  other: "dialog-other",
};

function openDialog(id) {
  document.querySelectorAll("dialog[open]").forEach((d) => d.close());
  const dialog = document.getElementById(id);
  if (dialog && !dialog.open) dialog.showModal();
}

document.querySelectorAll("[data-download]").forEach((button) => {
  button.addEventListener("click", () =>
    openDialog(DIALOG_FOR_PLATFORM[platform]),
  );
});

document.querySelectorAll("[data-open-dialog]").forEach((button) => {
  button.addEventListener("click", () => openDialog(button.dataset.openDialog));
});

document.querySelectorAll("[data-close-dialog]").forEach((link) => {
  link.addEventListener("click", () => link.closest("dialog")?.close());
});

// Clicking the backdrop closes a dialog.
document.querySelectorAll("dialog").forEach((dialog) => {
  dialog.addEventListener("click", (event) => {
    if (event.target !== dialog) return;
    const rect = dialog.getBoundingClientRect();
    const inside =
      event.clientX >= rect.left &&
      event.clientX <= rect.right &&
      event.clientY >= rect.top &&
      event.clientY <= rect.bottom;
    if (!inside) dialog.close();
  });
});

/* Mobile menu ---------------------------------------------------------------- */

const menu = document.querySelector("[data-mobile-menu]");
if (menu) {
  menu
    .querySelectorAll("a")
    .forEach((link) =>
      link.addEventListener("click", () => menu.removeAttribute("open")),
    );
  document.addEventListener("click", (event) => {
    if (menu.open && !menu.contains(event.target)) menu.removeAttribute("open");
  });
  menu.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && menu.open) {
      menu.removeAttribute("open");
      menu.querySelector("summary").focus();
    }
  });
}

/* Copy --------------------------------------------------------------------- */

const live = document.querySelector("[data-live]");

async function copyText(text) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    // Older browsers or insecure contexts: fall back to a hidden textarea.
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.opacity = "0";
    document.body.append(area);
    area.select();
    const ok = document.execCommand("copy");
    area.remove();
    return ok;
  }
}

function flash(button, text) {
  const original = button.dataset.label || button.textContent;
  button.dataset.label = original;
  button.textContent = text;
  button.classList.add("is-copied");
  if (live) live.textContent = text;
  clearTimeout(button._reset);
  button._reset = setTimeout(() => {
    button.textContent = original;
    button.classList.remove("is-copied");
  }, 2000);
}

document.querySelectorAll("[data-copy]").forEach((button) => {
  button.addEventListener("click", async () => {
    // Copy exactly one line, even if markup whitespace ever spans lines.
    const command = button.parentElement
      .querySelector("[data-command]")
      .textContent.replace(/\s+/g, " ")
      .trim();
    flash(button, (await copyText(command)) ? "Copied" : "Select and copy");
  });
});

document.querySelectorAll("[data-copy-link]").forEach((button) => {
  button.addEventListener("click", async () => {
    const url = `${location.origin}/`;
    flash(button, (await copyText(url)) ? "Link copied" : url);
  });
});

/* Header ------------------------------------------------------------------- */

const header = document.querySelector(".site-header");
const onScroll = () =>
  header?.classList.toggle("is-scrolled", window.scrollY > 8);
onScroll();
window.addEventListener("scroll", onScroll, { passive: true });

/* Demo --------------------------------------------------------------------- */

// Each scene uses a real sample from hushtype's cleanup tests.
const SCENES = [
  {
    scene: "messages",
    title: "Messages",
    mode: "Messaging mode",
    said: "meet me at the library actually the cafe",
    typed: "Meet me at the cafe.",
  },
  {
    scene: "mail",
    title: "Mail",
    mode: "Email mode",
    said: "we need to update the docs fix the tests and tag the release",
    typed: "We need to update the docs, fix the tests, and tag the release.",
  },
  {
    scene: "slack",
    title: "Slack",
    mode: "Messaging mode",
    said: "send it Tuesday no Wednesday",
    typed: "Send it Wednesday.",
  },
];

const demo = document.querySelector("[data-demo]");

if (demo) {
  const q = (selector) => demo.querySelector(selector);
  const scenes = [...demo.querySelectorAll("[data-scene]")];
  const title = q("[data-window-title]");
  const chip = q("[data-mode-chip]");
  const pill = q("[data-pill]");
  const pillLabel = q("[data-pill-label]");
  const keys = q("[data-keys]");
  const heard = q("[data-heard]");

  const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
  let visible = true;
  let wake = null;

  const onScreen = () => visible && !document.hidden;

  // Resolves after `ms`, but holds still while the demo is off screen.
  const wait = (ms) =>
    new Promise((resolve) => {
      const start = () => setTimeout(finish, ms);
      const finish = () => (onScreen() ? resolve() : (wake = resolve));
      if (onScreen()) start();
      else wake = start;
    });

  const resume = () => {
    const fn = wake;
    wake = null;
    fn?.();
  };

  new IntersectionObserver(([entry]) => {
    visible = entry.isIntersecting;
    demo.classList.toggle("is-paused", !visible);
    if (visible) resume();
  }).observe(demo);
  document.addEventListener("visibilitychange", () => {
    demo.classList.toggle("is-paused", document.hidden);
    if (!document.hidden) resume();
  });

  const typedIn = (name) =>
    demo.querySelector(`[data-scene="${name}"] [data-typed]`);

  const showScene = (s) => {
    scenes.forEach((el) =>
      el.classList.toggle("is-active", el.dataset.scene === s.scene),
    );
    title.textContent = s.title;
    chip.textContent = s.mode;
    typedIn(s.scene).textContent = "";
    heard.replaceChildren(
      ...s.said.split(" ").map((word, i, all) => {
        const span = document.createElement("span");
        span.textContent = i < all.length - 1 ? `${word} ` : word;
        return span;
      }),
    );
  };

  const setPill = (state, label) => {
    pill.dataset.state = state;
    pillLabel.textContent = label;
    pill.classList.toggle("is-visible", state !== "hidden");
  };

  const finalFrame = (s) => {
    showScene(s);
    heard
      .querySelectorAll("span")
      .forEach((span) => span.classList.add("is-said"));
    typedIn(s.scene).textContent = s.typed;
    setPill("hidden", "");
  };

  async function play() {
    for (let i = 0; ; i = (i + 1) % SCENES.length) {
      const s = SCENES[i];
      showScene(s);
      setPill("hidden", "");
      await wait(1100);

      keys.classList.add("is-down");
      setPill("listening", "Listening");
      for (const span of heard.querySelectorAll("span")) {
        await wait(230);
        span.classList.add("is-said");
      }
      await wait(450);

      keys.classList.remove("is-down");
      setPill("working", "Transcribing");
      await wait(450);
      setPill("working", "Refining");
      await wait(650);

      const typed = typedIn(s.scene);
      typed.textContent = s.typed;
      typed.classList.add("is-fresh");
      setPill("hidden", "");
      await wait(60);
      typed.classList.remove("is-fresh");
      await wait(3200);
    }
  }

  if (reduceMotion.matches) {
    finalFrame(SCENES[0]);
  } else {
    play();
  }
}
