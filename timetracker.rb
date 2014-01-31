#!/usr/bin/ruby
#

require 'rubygems'

require 'gli'

def printTimeDiff(start)
    finish = Time.now()
    seconds = finish - start
    print "\n\t**** The time difference: "
    print "%02d:%02d:%02d ****\n" % [
        seconds / (60*60),
        seconds / 60 % 60,
        seconds % 60
    ]
    print "\nThe finish time: #{finish}\n"
end
        

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
