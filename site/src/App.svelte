<script lang="ts">
  import Faq from "./components/Faq.svelte";
  import Features from "./components/Features.svelte";
  import Footer from "./components/Footer.svelte";
  import GetHams from "./components/GetHams.svelte";
  import Hero from "./components/Hero.svelte";
  import Keys from "./components/Keys.svelte";
  import Nav from "./components/Nav.svelte";
  import Privacy from "./components/Privacy.svelte";
  import Stages from "./components/Stages.svelte";
  import TryIt from "./components/TryIt.svelte";
</script>

<Nav />
<main id="top">
  <Hero />
  <TryIt />
  <Features />
  <Privacy />
  <Stages />
  <Keys />
  <Faq />
  <GetHams />
</main>
<Footer />
