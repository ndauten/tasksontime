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

    def timeDiffNowStr
        seconds = Time.now - @startTime
        "%02d:%02d:%02d" % [
            seconds / (60*60),
            seconds / 60 % 60,
            seconds % 60
        ]
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

module TimeUtils
    def TimeUtils.timeDiffPretty(seconds)
        "%02d:%02d:%02d" % [
            seconds / (60*60),
            seconds / 60 % 60,
            seconds % 60
        ]
    end

    # This function determines if the given time object occured since the most
    # recent sunday.
    def TimeUtils.sinceSunday(time)
        # Get a time object for right now
        now = Time.new

        # Get the date time object for the most recent sunday 12 am
        sunday = Time.new(now.year, now.month, now.day - now.wday, 0, 0, 0, 0)

        # Do the comparison and return the result 
        #print "Comparing: Last Sunday: #{sunday}, time: #{time}, compare: #{time>sunday}\n"

        time > sunday
    end
end
