<script lang="ts">
  import { PRIVACY, REPO } from "../lib/content";
  import { lines, stagger } from "../lib/motion";
</script>

<footer class="footer">
  <div class="footer-top" use:stagger={{ gap: 70 }}>
    <p>Hams 0.2 for macOS</p>
    <p>Voice typing that stays on your Mac</p>
    <a class="link" href="#top">Back to the top</a>
  </div>
  <a class="display footer-line" href="{REPO}/issues" use:lines={{ gap: 90 }}>{#each ["Say", "hello", "on", "GitHub"] as word, i (word)}{i ? " " : ""}<span class="line"><span>{word}</span></span>{/each}</a>
  <div class="footer-meta" use:stagger={{ gap: 70, from: 4, margin: "0px" }}>
    <p><a class="link" href="{REPO}/releases">All releases</a> · <a class="link" href="{REPO}/issues/new">Report a problem</a> · <a class="link" href={PRIVACY}>Privacy</a></p>
    <p>Speech by whisper.cpp · Built with Tauri</p>
    <p>© 2026 <a class="link" href="https://wusla.co">Wusla</a></p>
  </div>
</footer>
