<script lang="ts">
  // The closing choice: the DMG, or one line in Terminal that installs Hams
  // without the "can't be checked" warning.
  import { DMG, INSTALL_COMMAND, REPO } from "../lib/content";
  import { lines, onceInView, stagger, typeOut } from "../lib/motion";

  // The Terminal line types itself out once, the first time it's on screen.
  let shown = $state(INSTALL_COMMAND);
  let line: HTMLElement;
  $effect(() => onceInView(line, () => typeOut(INSTALL_COMMAND, (s) => (shown = s), { speed: 26 })));

  let copied = $state(false);
  async function copy() {
    try {
      await navigator.clipboard.writeText(INSTALL_COMMAND);
      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch {
      // No clipboard access: the line is selectable text anyway.
    }
  }
</script>

<section id="get" class="cta" aria-labelledby="cta-title">
  <h2 id="cta-title" class="display cta-title" use:lines={{ gap: 160 }}><span class="line"><span>Ready</span></span> <span class="line"><span>to talk?</span></span></h2>
  <div class="cta-choice" use:stagger={{ gap: 90, from: 2 }}>
    <div class="cta-row">
      <a class="button" href={DMG}>Download for Mac</a>
      <span class="or">or</span>
      <button type="button" class="button light" onclick={copy}>{copied ? "Copied" : "Copy the Terminal line"}</button>
    </div>
    <pre class="install" bind:this={line} aria-label={INSTALL_COMMAND}><code>{shown}<span class="caret"></span></code></pre>
    <p>
      The Terminal line installs Hams into Applications and opens it, with no warning from macOS. Free, no account, for any Mac on macOS 12 or later.
      <a class="link" href={REPO}>Read the code</a>.
    </p>
  </div>
</section>
