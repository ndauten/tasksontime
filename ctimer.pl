#!/usr/bin/perl
use strict;
use warnings;
use IO::Select;
use POSIX qw(strftime);
use Mail::Sendmail;
use Getopt::Long;
use DateTime;

#===============================================================================
# Support Functions 
#===============================================================================

sub usage(){
    print "\n";
    print "Usage: ./ctimer.pl [options] minutes\n\n";
    print "\t-f pomodoro_count_file (default: running_pomo_count.txt)\n";
    print "\t-b specify that this timer is a break\n";
    print "\n";
    exit;
}

sub mail_notification(){
    my $now_string = shift(@_);
    print "Mailing notification of completion\n\n";
    my %mail = ( To      => 'nathan.dautenhahn@intel.com',
        From    => 'nathan.dautenhahn@intel.com',
        Message => "Pomodoro Completed!",
        Subject => "Pomodoro Completed at $now_string"
    );
    sendmail(%mail) or die $Mail::Sendmail::error;
    #print "OK. Log says:\n", $Mail::Sendmail::log;
}

# Add to the running count of pomodoros
sub count_pomodoro(){
    my $running_fn = shift(@_);
    my $count = 0;
    my $trash = 0;
    if (-e $running_fn) {
        open FH, $running_fn; ($count,$trash) = split('\n',<FH>); close FH; 
    }
    open FH, ">$running_fn"; $count +=1; print FH $count;
    print "You have completed $count pomodoros\n";
}

#===============================================================================
# Main script
#===============================================================================
#### OPTIONS ####
my $pomo_count_file = 'running_pomo_count.txt';
my $help = '';
my $minutes = 0;
my $break = 0;

GetOptions (
        'help' => \$help, 
        'file=s' => \$pomo_count_file, 
        'break' => \$break
    );

&usage if($help);
&usage if(!$ARGV[0]);
$minutes = $ARGV[0];

my $s = IO::Select->new();
$s->add(\*STDIN);
my $now_string = strftime "%a %b %e %H:%M:%S %Y", localtime;
my $duration = $minutes * 60; # duration in seconds means input in minutes
my $beg_time = time;
my $end_time = $beg_time + $duration; 
my $input = "";
print "Starting timer for $minutes minutes on $now_string\n\n";
for (;;) {
  $|++;
  my $time = time;
  last if ($time >= $end_time);
  printf("\r\t[ %02d:%02d:%02d ] -- command [f,p,q]? %s",
     ($end_time - $time) / (60*60),
     ($end_time - $time) / (   60) % 60,
     ($end_time - $time)           % 60,
     $input
  );
  if($s->can_read(.1)){
    chomp(my $input = <STDIN>);
    #print "Got '$foo' from STDIN\n";
    #chomp($foo = <STDIN>);
    last if($input eq 'f');
    if($input eq 'p'){
        $now_string = strftime "%H:%M:%S", localtime;
        print "\tPaused: $now_string ... Press enter to continue.";
        my $begin_pause = time;
        chomp($input = <STDIN>);
        $now_string = strftime "%H:%M:%S", localtime;
        print "\tResuming... $now_string\n";
        $end_time+=time - $begin_pause;
    }
    if($input eq 'q'){
        print "\nExiting without counting the pomodoro\n\n";
        exit();
    }
  }
}

$now_string = strftime "%a %b %e %H:%M:%S %Y", localtime;
# The \a is a system bell so it sounds even when I'm logged remotely
print "\nFinished at $now_string.\a\n";

print "Would you like to count the pomodoro and start another? [Y/n]\n";

##
# I want it to automatically caclulate the next pomodoro begin time here. 
# Outline of code: 
#
# while (1){
#   do_pomodoro();
#   ask_for_action_and_get_choice();
#   if( count_and_continue ){
#       count_pomodoro();
#       continue;
#   } elsif (count_and_quit){
#       count_pomodoro();
#       last;
#   } elsif( void_and_restart ){
#       continue;
#   } elsif (void_and_quit){
#       last;
#   }
# }
##

#foreach my $i (1 .. 190000){print("KABOOM!!!!");}
#system('./asciiquarium');

if(!$break) {
    if (`uname` ne "Linux"){
        &mail_notification($now_string);
        system("DISPLAY=:2.0 ./naughty-notify.sh");
        system("DISPLAY=:1.0 ./naughty-notify.sh");
    } else {    
        system('osascript ./mac_growl_notify.scpt');
        #system('say "You are done! Successful completion of Pomodoro!"');
    }
    &count_pomodoro($pomo_count_file);
}
