<script lang="ts">
  import { COMMANDS } from "../lib/content";
  import { reveal } from "../lib/reveal";
  import Pill from "./Pill.svelte";
</script>

<section class="keys" aria-labelledby="keys-title">
  <h2 id="keys-title" class="head keys-title" use:reveal>Everything you can press or say</h2>
  <div class="keys-body">
    <figure class="pills" data-ui use:reveal aria-label="The Hams pill while recording, while writing, and when done">
      <Pill state={{ kind: "recording", time: "0:12" }} />
      <Pill state={{ kind: "working", label: "Polishing" }} />
      <Pill state={{ kind: "done", label: "Pasted into Notes" }} />
      <figcaption>The pill sits at the bottom of your screen and never takes the focus from your app.</figcaption>
    </figure>
    <table class="commands">
      <caption class="visually-hidden">Shortcuts and voice commands</caption>
      <tbody>
        {#each COMMANDS as command, i (i)}
          <tr class:group={command.group}>
            <th scope="row">{command.kind}</th>
            <td>
              {#if command.prefix}{command.prefix}{" "}{/if}{#each command.keys ?? [] as key (key)}<kbd>{key}</kbd>{/each}{command.say ?? ""}
            </td>
            <td>{command.does}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>
</section>
