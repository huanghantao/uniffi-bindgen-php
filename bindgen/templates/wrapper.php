<?php

{%- for obj in ci.object_definitions() %}
"{{ obj|type_name }}"
{%- endfor %}

{% import "macros.php" as php %}
