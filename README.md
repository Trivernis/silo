# silo

Silo is a dotfile manager that supports templating.

## Install

Currently silo can only be installed manually by cloning the repo and running `cargo install --path .`

## Usage

### Create Repo

First create a repo

```nu
silo --repo /path/to/repo init
```
This creates the repo directory and initializes a git repository.
If no `--repo` argument is passed, it will default to `$HOME/.local/share/silo` or `$HOME/AppData/Roaming/silo`.

### Add configuration files

Now add some configuration files you want to track.
Silo uses metadata-files to keep track of which files belong where.
For example if you want all files in the root directory of your repo to be copied over
to your home folder, you'd add a `dir.toml` entry like this:

```toml
path = "{{dirs.home}}"
ignored = []
```

Notice the use of templating for the path. The `dirs` variable contains paths specific to your platform.
`home` in this case would either be `{FOLDERID_Profile}` on Windows or `$HOME` on Linux and MacOS.
The `ignored` setting can be used to ignore certain files using an array of glob-strings.

Now add some files to the repos root directory.
Normal files get just copied over. Subdirectories are created and copied as well, unless they themselves
contain a `dirs.toml` file that specifies a different location. 

Files ending with `.tmpl` are treated as [handlebars templates](https://handlebarsjs.com/) and are processed
before being written to the target location. The `.tmpl` extension will be stripped from the filename.
You can check the available context variables and their values on the system with `silo context`.


### Applying the configuration

Once you have a repo you want to apply you can run 
```nu
silo --repo /path/to/repo apply
```
which will process and copy over all the configuration files of that repository.


### Configuring Silo

Silo has several configuration files that are applied in the following order:

- `~/.config/silo.toml`  (or the equivalent on windows)
- `repo.toml` in the repo's folder
- `repo.local.toml` in the repo's folder (specific to the system. Don't commit this file)
- environment variables with prefix `SILO_`

A configuration file looks like this (with all the defaults):

```toml
# The diff tool that is being used when displaying changes and prompting for confirmation
diff_tool = "diff"

# Additional context that is available in all handlebar templates under the `ctx` variable
[template_context]
# hello = "world"
```


### License

CNPL-v7+
