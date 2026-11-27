# earnest_quint_syntax

A Quint file read into what `quint parse` writes, with no Quint and no Node.

`read` takes the path of a `.qnt` file and a way to load files, and gives `Parsed`: the modules
of that file and of every file it imports, as the JSON objects Quint 0.33.1 writes in `modules`,
and the source map `quint parse --source-map` writes. The ids are Quint's own. A file Quint
refuses gives a `Problem` with Quint's code, line and column.

It does Quint's parse phases 1 and 2: the text to modules with every rewrite Quint's parser makes,
then the imported files read with the same ids and the modules sorted by their imports. It does not
resolve names: there is no `table`, no `depth` on `const` and `var`, and a module's declarations
stay in the order they are written. Nothing in earnest calls it yet.

The tests compare it with recordings of Quint 0.33.1 for every `.qnt` file in the repository.
`tasks/quint/README.md` has what is compared, the rewrites, the results and how to record again.
