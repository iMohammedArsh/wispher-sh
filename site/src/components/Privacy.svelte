<script lang="ts">
  import { reveal } from "../lib/reveal";
</script>

<section id="privacy" class="privacy" aria-labelledby="privacy-title">
  <ul class="facts" use:reveal>
    <li>Works offline</li>
    <li>No account</li>
    <li>Open source</li>
  </ul>
  <div class="privacy-text">
    <h2 id="privacy-title" class="head" use:reveal>Your voice stays on your Mac</h2>
    <p class="statement" use:reveal={1}>
      <img class="dropcap" src="img/icon.png" width="256" height="256" alt="The Hams app icon" /><strong>Hams</strong> (همس) is Arabic for a
      whisper. Your voice is heard by Whisper, the open speech model, running inside the app on your Mac’s own chip. Nothing is recorded to
      keep, nothing is uploaded, and it works on a plane.
    </p>
    <p class="statement soft" use:reveal={2}>
      AI polish is optional. Turn it on and only the written text goes to the provider you pick, with your own key, stored in the Mac’s
      keychain. Or run it locally with Ollama.
    </p>
  </div>
</section>
