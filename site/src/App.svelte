<script lang="ts">
  import Board from "./components/Board.svelte";
  import Faq from "./components/Faq.svelte";
  import Footer from "./components/Footer.svelte";
  import GetHams from "./components/GetHams.svelte";
  import Hero from "./components/Hero.svelte";
  import HowItWorks from "./components/HowItWorks.svelte";
  import Keys from "./components/Keys.svelte";
  import Nav from "./components/Nav.svelte";
  import Privacy from "./components/Privacy.svelte";
  import Switches from "./components/Switches.svelte";
  import TryIt from "./components/TryIt.svelte";
</script>

<Nav />
<main id="top">
  <Hero />
  <TryIt />
  <Board />
  <Switches />
  <HowItWorks />
  <Privacy />
  <Keys />
  <Faq />
  <GetHams />
</main>
<Footer />
