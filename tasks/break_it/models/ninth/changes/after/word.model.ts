import { m } from "@earnestjs/model";

export const Word = m.model("Word", {
	text: m.oneOf(["a b", "a  b"]).initial("a b"),
	done: m.boolean().initial(false),
});

export const say = m.action(Word, "the word is said", {
	when: (s) => !s.done,
	changes: () => ({ done: true }),
});

export const said = m.end(Word, "the word was said", (s) => s.done);

export const spaced = m.eventually(Word, "the word comes to have one space", {
	reaches: (s) => s.text === "a  b",
});

export const single = m.setup(Word, "the word starts with one space", { text: "a  b" });

export const once = m.witness(Word, "the word is said with one space", {
	steps: [say],
	expect: (s) => s.text === "a  b",
});
