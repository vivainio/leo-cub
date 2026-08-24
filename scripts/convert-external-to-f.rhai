// Recursively finds every `.leo` file under a directory and renames each
// external-file node (`@auto`, `@auto-md`, `@auto-markdown`, `@file`,
// `@thin`, `@file-thin`, `@clean`) to the equivalent `@f <path>` node, then
// saves. This promotes every derived/thin file leo-cub recognizes to its
// own cub-1-thin sentinel format, so the resulting tree can be diffed
// against the pre-conversion checkout to see exactly what changes.
//
// Usage: cub run scripts/convert-external-to-f.rhai <directory>

if ARGS.len() == 0 {
    print("usage: cub run convert-external-to-f.rhai <directory>");
    throw "missing directory argument";
}

let root = ARGS[0];

// Directives leo-cub treats as external-file nodes, per `external_file()`
// in src/inspect.rs -- everything but `@f` itself, since that's the target
// format. Matching "<directive>(\s|$)" (not a bare prefix) keeps `@auto`
// from also matching `@auto-md`/`@auto-markdown` headlines.
const DIRECTIVES = ["@file-thin", "@auto-markdown", "@auto-md", "@auto", "@file", "@thin", "@clean"];

let find = sh("find '" + root + "' -iname '*.leo'");
assert(find.code == 0, "find failed: " + find.stderr);

let files = find.stdout.split("\n");
let total_files = 0;
let total_nodes = 0;

for file in files {
    file.trim();
    if file == "" {
        continue;
    }

    let doc = open(file);
    let converted = 0;

    for directive in DIRECTIVES {
        let pattern = "^" + directive + "(\\s|$)";
        for node in doc.find_h(pattern) {
            let h = node.h;
            let rest = h.sub_string(directive.len());
            let filename = rest;
            filename.trim();
            if filename == "" {
                // Bare directive, no filename -- not a real external node.
                continue;
            }
            node.h = "@f" + rest;
            converted += 1;
        }
    }

    if converted > 0 {
        doc.save();
        total_files += 1;
        total_nodes += converted;
        print(file + ": converted " + converted + " node(s)");
    }
}

print("done: " + total_nodes + " node(s) across " + total_files + " file(s)");
