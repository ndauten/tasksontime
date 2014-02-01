#!/usr/bin/ruby
#

class Timer
    attr_reader :startTime, :endTime
    def initialize()
        @startTime = Time.now
    end

    def start
        @startTime = Time.now
    end

    def stop
        @endTime = Time.now
    end

    def printTimeDiffNow
        seconds = Time.now - @startTime
        print "\n\t**** The time difference: "
        print "%02d:%02d:%02d ****\n" % [
            seconds / (60*60),
            seconds / 60 % 60,
            seconds % 60
        ]
    end

    def timeDiffSecs
        @endTime - @startTime
    end
end
