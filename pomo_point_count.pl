#!/usr/bin/perl -w
use strict;
use DateTime;
use POSIX qw(strftime);
use Getopt::Long;

my ($points, $pomos, $plan) = (0,0,0);
my ($tpoints, $tpomos, $tplan, $texp) = (0,0,0);
my %tags;
my @days = qw/Sunday Monday Tuesday Wednesday Thursday Friday Saturday/;

my $dt = DateTime->now()->subtract( hours => 6);

sub backup(){
    print "Backing up to stats file: $_[0]\n\n";
    system("cp ./stats.txt stats.archive");
    #open STATS,">>",$_[0] or die "\n\n!!! non existent file !!! $1\n\n";
}

#### OPTIONS ####
my $today = $dt->day_name; 
my $file = 'today.txt';
my $help = '';
my $summary = '';
my $write = '';
my $backup = '';

GetOptions ('summary' => \$summary, 
            'help' => \$help, 
            'file=s' => \$file, 
            'day=s' => \$today,
            'write' => \$write,
            'backup' => \$backup
);

if($summary){$today="all";}
if($help){
    print "\n";
    print "Usage: ./pomo_point_count.pl -f todo_file (default: today.txt)";
    print "\n\n";
    exit;
}elsif($backup){
    &backup("./stats.txt");
}

### Time Init and Print Out Week to Screen ###
my $now = localtime;
my $dayofweek = DateTime->now()->dow;
$dt->subtract( days => $dayofweek );
my $dtf = $dt->clone();
$dtf->add( days=>6 );
my $l1 = sprintf "|  Sunday %s.%s.%s ---> Saturday %s.%s.%s  |",
    $dt->month,$dt->day,$dt->year,$dtf->month,$dtf->day,$dtf->year;
my $duration = sprintf "%57s",$l1;
my $len = length($l1);
my $line = sprintf "\n%57s\n","+" . ('-' x ($len-2)) . "+";
print $line;
print $duration;
print $line;

# Function that parses the file, collects stats for the given day, and prints 
# format: [complete=X,unfinished=' '] Task [project|points|exppomos|pomos]
sub print_day_stats(){
    if( ! open TODOFH, $file){die "\n\n!!! non existent file !!!\n\n"};
    my ($exp,$eff,$flag);
    my $today = $_[0];
    while(<TODOFH>){
        # Match for total pomos expected for efficiency
        if(/===== Expected Weekly Pomos: ([\d]+)/) { $texp = $1; }
        # Match for counting pomos
        if(/===== (.*) \[[\d]*:([\d]+).*/){
            if($1 eq $today){
                $flag = 1;
                $exp=$2;
            }elsif($flag){
                if($exp > 0){
                    $eff = $pomos/$exp;
                }else{
                    $eff = "0.00"
                }
                printf "\t%2s %15d %16s\t%11.02f\n",$points,$plan,$pomos,$eff;
                $tplan += $plan; $tpoints += $points, $tpomos += $pomos; 
                #$texp += $exp;
                $plan = 0; $points = 0; $pomos = 0; 
                close TODOFH;
                return;
            }
        }
        if($flag){
            if(/\|(x+)\]/){
                $pomos+=length($1);
            }
            # Match for counting points
            if(/\[x\].*\[.*\|([[\d]+|\.[\d]+|\d\.[\d]+])\|.*\|.*\]/){ 
                $points+=int($1); 
            }
            # Match for counting planned pomos
            if(/\[.*\|.*\|([\d]+[\.[\d]*|""])\|.*\]/){
                $plan+=$1;
            }
            if(/\[.\].*\[(.*)\|.*\|.*\|(x+)\]/){
                my $p;
                if(length($1)){
                    $p = $1;
                }else{
                    $p = "noname";
                }
                if(exists($tags{$p}))
                {
                    my $newct = $tags{$p} + length($2);
                    $tags{$p} = $newct;
                }else{
                    $tags{$p} = length($2);
                }
            }
        }
    }
}

printf "\t%15s %15s %17s %12s\n", "Earned Points","Planned Pomos","Completed Pomos","Efficiency";
if($summary){
    foreach $today (@days){
        print "$today:  ";
        &print_day_stats($today);
    }
    printf "\nTotals: \t%2s %15s %16s\t%11.02f\n",$tpoints,$tplan,$tpomos,$tpomos/$texp;
}else{
    print "$today:  ";
    &print_day_stats($today);
    print "\n";
}
printf "\nProject Allocation:\n";
foreach my $_ (sort {$tags{$b} cmp $tags{$a}} keys %tags){
    printf("\n%15s: %2s/%s (%.02f%s)",$_,$tags{$_},$tpomos,$tags{$_}/$tpomos,'%');
}
# to sort values use keys to lookup the values and a compare block to compare
# them
print "\n"
