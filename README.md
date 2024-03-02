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

If you have an existing repo somewhere you can do
```nu
silo --repo /path/to/repo init <remote-url>
```
which will clone the remote repository to the given path.

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

Now add some files to a directory `content` in the repo.
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


### Advanced

#### File permissions

File permissions are persisted the way git stored them. This is true for templates as well. So a template with 
execute permission will result in a rendered file with the same permission.


#### Hooks

All `.nu` files in the `hooks` folder in the repos root are interpreted as hook scripts.
Currently there's four functions that can be defined in these scripts that correspond to 
events of the same name:
```
before_apply_all 
after_apply_all
before_apply_each  
after_apply_each  
```
These functions will be called with a single argument, the event context, that can be used
to change certain properties of files or inspect the entire list of files that are about to be written.
For example one could change the attributes of script files with the following hook

```nu
# Make `test-2/main` executable
def after_apply_each [ctx] {
  if $ctx.dst =~ "test-2/main" {
    chmod +x $ctx.dst
  }
}
```

### License

CNPL-v7+
