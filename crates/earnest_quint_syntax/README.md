# earnest_quint_syntax

A Quint file read into what `quint parse` writes, with no Quint and no Node.

`read` takes the path of a `.qnt` file and a way to load files, and gives `Parsed`: the modules
of that file and of every file it imports, as the JSON objects Quint 0.33.1 writes in `modules`,
and the source map `quint parse --source-map` writes. The ids are Quint's own. A file Quint
refuses gives a `Problem` with Quint's code, line and column.

It does all four of Quint's parse phases: the text to modules with every rewrite Quint's parser
makes; the imported files read with the same ids and the modules sorted by their imports; names
resolved into the `table`, with `depth` on constants, variables, types and assumptions; and each
module's declarations sorted by what they use. `Parsed::output` is the whole of what
`quint parse --out` writes, and a file Quint's name phases refuse gives Quint's code, place and
message. Nothing in earnest calls it yet.

The tests compare it with recordings of Quint 0.33.1 for every `.qnt` file in the repository.
`tasks/quint/README.md` has what is compared, the rewrites, the results and how to record again.
