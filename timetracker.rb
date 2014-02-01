#!/usr/bin/ruby
#

require 'rubygems'

require 'gli'

require './brick_tree'

include GLI::App

program_desc 'Track, manage, and query a task tree where each node is a potential project or task'

accept(Date) do |string|
      Date.parse(string)
end

desc 'Specify the file where the task tree lives'
arg_name 'path'
default_value File.join(ENV['HOME'],'.tasktree.yml')
flag [:t,:tasktree]


desc 'Be verbose'
switch 'verbose'


desc 'List task tree'
long_desc <<EOS
List the tasks in your task tree, possibly including time worked.  By default,
this will list all tree nodes and not other information.
EOS

command [:show,:sh,:list,:ls] do |c|

    c.desc 'Show tree with names'
    c.command :tree do |all|
        all.action do 
            Claret::TaskListTerminalSerializer.new(:all).write($task_list)
        end
    end

    c.default_command :tree
end


exit run(ARGV)


#def printTimeDiff(start)
#    finish = Time.now()
#    seconds = finish - start
#    print "\n\t**** The time difference: "
#    print "%02d:%02d:%02d ****\n" % [
#        seconds / (60*60),
#        seconds / 60 % 60,
#        seconds % 60
#    ]
#    print "\nThe finish time: #{finish}\n"
#end
        

#print "\tCommand [r]: "
#while(gets.chomp() =~ /rh/) do
#end
#puts "Starting..."
#begin
    #start = Time.now()
    #puts start
    #print "\tCommand [r]: "
    #while(gets.chomp() =~ /r/) do
        #printTimeDiff(start)
        #puts "\nRestarting..."
        #start = Time.now()
        #puts start
    #end
    #printTimeDiff(start)
#rescue Interrupt => e
    #print "\n"
    #printTimeDiff(start)
    #exit
#end
