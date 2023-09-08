#===============================================================================

# File:     README

# Author:   Nathan Dautenhahn

# Date:     January 31, 2014

#===============================================================================

<=== Overview ===>

This repository contains several scripts that I have used to do time management
tasks. I plan to eventually incorporate them all into a single application, but
they are each experiements, some with mature development and others with minimal
development.

<=== Sub App Descriptions ===>
    - ctimer.pl:
        This is a perl script that captures the pomodoro style time tracking.
    - timetracker:
        This application is focused on tracking time as it relates to
        roles/projects/tasks. It uses YAML input/output to file in order to
        store the data.

    - Notes on install, while in dev:
            - Install bundler:

    gem install bundler

    - Install Dependencies of bunderl

    bundle install

    - Note this required me to have the dev libs for ruby on my machine

    - Execute timetracker with:

    bundle exec bin/timetracker

    - The last command will print out the help style interface for the
              application. Get instructions there for how to use it.

<=== How to use ===>

1. The default yaml file will be at ``ENV['HOME'],'.bricktree.yml'``
2. Add a brick to timetracker
   ```tt add -b (BRICK_NAME) -p (PARENT_NAME)```
3. Record time
   ```tt re -b (BRICK_NAME)```
   Then in the subshell you got
   ```Command: r: reset, c: complete, q: quit``` to end timer.