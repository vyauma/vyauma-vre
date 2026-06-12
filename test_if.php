<?php
$a = 10;
$b = 20;

if ($a < $b) {
    echo "10 is less than 20\n";
} else {
    echo "Unreachable\n";
}

if ($b < $a) {
    echo "Unreachable\n";
} else {
    echo "20 is not less than 10\n";
}
?>
