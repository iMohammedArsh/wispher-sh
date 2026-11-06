<script lang="ts">
  // The closing choice: the DMG, or one line in Terminal that installs Hams
  // without the "can't be checked" warning.
  import { DMG, INSTALL_COMMAND, REPO } from "../lib/content";

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
  <h2 id="cta-title" class="display cta-title"><span>Ready</span> <span>to talk?</span></h2>
  <div class="cta-choice">
    <div class="cta-row">
      <a class="button light" href={DMG}>Download for Mac</a>
      <span class="or">or</span>
      <button type="button" class="button green" onclick={copy}>{copied ? "Copied" : "Copy the Terminal line"}</button>
    </div>
    <pre class="install"><code>{INSTALL_COMMAND}</code></pre>
    <p>
      The Terminal line installs Hams into Applications and opens it, with no warning from macOS. Free, no account, for any Mac on macOS 12 or later.
      <a class="link" href={REPO}>Read the code</a>.
    </p>
  </div>
</section>
