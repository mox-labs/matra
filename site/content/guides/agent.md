# Hand matra to an agent

Give an agent matra's own instructions, printed by the version it will run, rather than pointing it at these pages.

## Print the skill

If the agent has a shell and nothing installed, give it this line:

```bash
uvx 'matra>=0.2' --skill
```

If matra is already installed, `matra --skill` prints the same text. It says what matra is for, when to reach for it, every command with its JSON shape, how to read each number and what it does not license the agent to conclude, the limits, and the errors.

Keep the version floor in the `uvx` line. `--skill` arrived in 0.2.0, and an unpinned `uvx matra` takes whatever release is newest, which before 0.2.0 was a different command line without the flag.

## Give it the deeper references

```bash
matra --skill -r          # the references, one per line: name, then summary
matra --skill -r json     # one reference
```

The text is compiled into the binary and the wheel, so it matches the version that is running, and the test suite executes every command in it against the command line. The [CLI reference](cli.md#for-an-agent) has the flag's exit codes and its `--json` shapes.

## Load it as a plugin

The same files install as a Claude Code plugin. Run `claude --plugin-dir <checkout>` over a clone of the repository, and plugin discovery picks up `skills/matra/`.

## Point a reader of this site at llms.txt

An agent reading the documentation instead of running matra starts at [`llms.txt`](https://mox-labs.github.io/matra/llms.txt), served at the site's root: every page with its one-line summary.
