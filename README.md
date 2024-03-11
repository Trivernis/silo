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
to your home folder, you'd add a `silo.dir.lua` entry like this:

```lua
local silo = require 'silo'

return { 
  path = silo.dirs.home,
  -- defaults to "exclude". Can be "include" to only look at included paths
  mode = "exclude",
  -- excluded glob patterns if mode is "exclude"
  exclude = {},
  -- included glob patterns if mode is "include"
  include = {}
}
```

The `silo` module provides utility functions and values that can be used in configuration files.
You can print those while evaluating the config files by using the `log` module:

```lua
local silo = require 'silo'
local log = require 'log'

log.debug(silo) -- debug prints the input value serialized as json

return { 
  path = silo.dirs.home,
}
```

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

- `~/.config/silo.config.lua`  (or the equivalent on windows)
- `silo.config.lua` in the repo's folder
- environment variables with prefix `SILO_`

A configuration file looks like this (with all the defaults):

```lua
local silo = require 'silo'
local config = silo.default_config

-- The diff tool that is being used when displaying changes and prompting for confirmation
config.diff_tool =  "diff"

-- Additional context that is available in all handlebar templates under the `ctx` variable
config.hello = "world"

return config
```


### Advanced

#### File permissions

File permissions are persisted the way git stored them. This is true for templates as well. So a template with 
execute permission will result in a rendered file with the same permission.


#### Hooks

All `.hook.lua` files in the `hooks` folder in the repos root are interpreted as hook scripts.
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

```lua
local utils = require 'utils'
local chmod = utils.ext 'chmod'

return {
  -- Make `test-2/main` executable
  after_apply_each = function(ctx)
    local fname = "test-2/main"
    if string.sub(ctx.dst, -#fname) == fname then
      chmod {"+x", ctx.dst}
    end
  end
}
```

### License

CNPL-v7+
