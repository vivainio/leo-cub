# Work through a tutorial outline

This page uses [`tutorial.leo`](https://github.com/vivainio/leo-cub/blob/main/tutorial.leo)
as a small, repeatable example. The commands below run during the Zensical
build, so the displayed outline is generated from the actual Leo file.

## Inspect the outline

The compact inspector includes positions, GNXs, headlines, and bodies:

```bash
target/release/cub inspect tutorial.leo
```

This is useful when diagnosing an outline, but it is intentionally dense. For
documentation, render only the hierarchy:

## Render the hierarchy

```bash
target/release/cub render tutorial.leo
```

The outline renderer emits a nested Markdown list. A repeated vnode is shown as
`↪ clone`, so the `Tasks` entry under `Reference` makes the clone visible without
duplicating its descendants.

## Highlight a current section

Use an occurrence position to mark the section being discussed. The position
is occurrence-specific, which matters when a node is cloned:

```bash
target/release/cub render tutorial.leo --current 0/0/1
```

The current node and its ancestors receive CSS classes that the site theme can
highlight.

## Collapse the sample

For a compact book sidebar or a step-by-step tutorial, collapse unrelated
branches and open the path around the current section:

```bash
target/release/cub render tutorial.leo --collapsed --current 0/0/1 --expand 0/0/0
```

Collapsed output uses native HTML `<details>` elements. No JavaScript or saved
Leo UI state is needed, so the page remains deterministic in CI and stable when
the outline is edited.

## The rendered sample

This is the expanded output, included directly so the tutorial remains
previewable on Zensical versions without third-party build plugins:

<div class="leo-outline-sample">
<ul class="leo-outline__list">
  <li><span class="leo-outline__ancestor">Tutorial project</span>
    <ul class="leo-outline__list">
      <li><span class="leo-outline__ancestor">Project</span>
        <ul class="leo-outline__list">
          <li>Source
            <ul class="leo-outline__list">
              <li>main.rs</li>
              <li>lib.rs</li>
              <li class="leo-outline__last">tests.rs</li>
            </ul>
          </li>
          <li><span class="leo-outline__current" data-position="0/0/1" aria-current="page">Documentation <span class="leo-current-label">current</span></span>
            <ul class="leo-outline__list">
              <li>Tutorial</li>
              <li>Concepts</li>
              <li class="leo-outline__last">Command reference</li>
            </ul>
          </li>
          <li class="leo-outline__last">Tasks
            <ul class="leo-outline__list">
              <li>Write the first draft</li>
              <li>Review the examples</li>
              <li class="leo-outline__last">Publish the guide</li>
            </ul>
          </li>
        </ul>
      </li>
      <li class="leo-outline__last">Reference
        <ul class="leo-outline__list">
          <li>Tasks <span class="leo-clone-label">↪ clone</span></li>
          <li>Concepts <span class="leo-clone-label">↪ clone</span></li>
          <li class="leo-outline__last">Command reference <span class="leo-clone-label">↪ clone</span></li>
        </ul>
      </li>
    </ul>
  </li>
</ul>
</div>
